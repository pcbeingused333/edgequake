//! Image size guard for vision LLM calls (SPEC-134 Slice D, WP-1).
//!
//! Acquisition law: the channel cannot transmit what the receiver rejects.
//! Providers enforce per-image payload limits (e.g. Anthropic ~5MB); a dense
//! manuscript page rendered at 300 DPI can exceed 10MB as PNG and is then
//! silently degraded or errors the call (measured 2026-08-20: an 11.9MB page
//! PNG produced an empty page with no error surfaced). This wrapper re-encodes
//! oversized images (PNG -> JPEG q85, then downscale) before they reach the
//! provider, so every vision call path (Pass-A OCR, Pass-B analyze, grounding
//! judge) inherits size safety from one place.

use std::sync::Arc;

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use edgequake_llm::traits::ImageData;
use edgequake_llm::{
    ChatMessage, CompletionOptions, LLMProvider, LLMResponse, Result, ToolChoice, ToolDefinition,
};
use tracing::warn;

/// Env override for the per-image binary size budget (bytes). `0` disables.
pub const MAX_IMAGE_BYTES_ENV: &str = "EDGEQUAKE_VISION_MAX_IMAGE_BYTES";

/// Default per-image budget: 3.5MB binary ≈ 4.7MB on the wire after base64
/// inflation — under the tightest well-known provider limit (5MB).
pub const DEFAULT_MAX_IMAGE_BYTES: usize = 3_500_000;

/// JPEG quality for re-encodes. q85 artifacts are invisible on high-contrast
/// ink; the signal (handwriting strokes) is preserved.
const JPEG_QUALITY: u8 = 85;

/// Downscale floor for print / Pass-B crops.
pub const PRINT_MIN_LONG_SIDE: u32 = 1024;
/// OmniDocBench v1.5 notes floor is 200 DPI; 2000px long-edge ≈ that on A4.
pub const MANUSCRIPT_MIN_LONG_SIDE: u32 = 2000;

/// Resolve the size budget from env; `0` disables the guard entirely.
pub fn max_image_bytes_from_env() -> usize {
    std::env::var(MAX_IMAGE_BYTES_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_IMAGE_BYTES)
}

/// Delegates to an inner provider while re-encoding oversized image payloads.
pub struct ImageGuardProvider {
    inner: Arc<dyn LLMProvider>,
    max_bytes: usize,
    min_long_side: u32,
}

impl ImageGuardProvider {
    /// Wrap `inner` with the size guard; returns `inner` unchanged when the
    /// guard is disabled via `EDGEQUAKE_VISION_MAX_IMAGE_BYTES=0`.
    pub fn wrap(inner: Arc<dyn LLMProvider>) -> Arc<dyn LLMProvider> {
        Self::wrap_with_min_long_side(inner, PRINT_MIN_LONG_SIDE)
    }

    /// SPEC-134 Slice E: manuscript Pass-A may JPEG but must not shrink below
    /// [`MANUSCRIPT_MIN_LONG_SIDE`].
    pub fn wrap_for_modality(
        inner: Arc<dyn LLMProvider>,
        modality: crate::page_modality::PageModality,
    ) -> Arc<dyn LLMProvider> {
        let floor = if modality.is_manuscript_like() {
            MANUSCRIPT_MIN_LONG_SIDE
        } else {
            PRINT_MIN_LONG_SIDE
        };
        Self::wrap_with_min_long_side(inner, floor)
    }

    pub fn wrap_with_min_long_side(
        inner: Arc<dyn LLMProvider>,
        min_long_side: u32,
    ) -> Arc<dyn LLMProvider> {
        let max_bytes = max_image_bytes_from_env();
        if max_bytes == 0 {
            return inner;
        }
        Arc::new(Self {
            inner,
            max_bytes,
            min_long_side: min_long_side.max(1),
        })
    }

    async fn guard_messages_async(&self, messages: &[ChatMessage]) -> Vec<ChatMessage> {
        // Image decode/encode is CPU-bound; keep it off the async executor.
        let max_bytes = self.max_bytes;
        let min_long_side = self.min_long_side;
        let owned: Vec<ChatMessage> = messages.to_vec();
        tokio::task::spawn_blocking(move || guard_messages(&owned, max_bytes, min_long_side))
            .await
            .unwrap_or_else(|_| messages.to_vec())
    }
}

/// gpt-5.x and o-series models only accept the default temperature.
/// edgequake-llm's `model_requires_default_temperature` also lists gpt-4.1-*,
/// which accept one, so it is not reused here: that would drop the OCR
/// temperature on gpt-4.1-mini.
fn model_requires_default_temperature(model: &str) -> bool {
    let model = model.trim();
    let model = model
        .rsplit('/')
        .next()
        .unwrap_or(model)
        .to_ascii_lowercase();
    model.contains("gpt-5")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
}

/// pdf2md page calls set `temperature: 0.1`; drop it for models that reject a
/// non-default temperature.
fn with_effective_temperature(model: &str, options: &CompletionOptions) -> CompletionOptions {
    let mut options = options.clone();
    if model_requires_default_temperature(model) {
        options.temperature = None;
    }
    options
}

fn guard_messages(
    messages: &[ChatMessage],
    max_bytes: usize,
    min_long_side: u32,
) -> Vec<ChatMessage> {
    messages
        .iter()
        .map(|msg| {
            let Some(images) = &msg.images else {
                return msg.clone();
            };
            if images.iter().all(|img| img.is_url()) {
                return msg.clone();
            }
            let mut guarded = msg.clone();
            guarded.images = Some(
                images
                    .iter()
                    .map(|img| guard_image(img, max_bytes, min_long_side))
                    .collect(),
            );
            guarded
        })
        .collect()
}

/// Re-encode one image when its decoded size exceeds `max_bytes`.
/// Pass-through for URLs, undecodable payloads, and images already in budget.
fn guard_image(img: &ImageData, max_bytes: usize, min_long_side: u32) -> ImageData {
    if img.is_url() {
        return img.clone();
    }
    let Ok(bytes) = B64.decode(&img.data) else {
        return img.clone();
    };
    if bytes.len() <= max_bytes {
        return img.clone();
    }
    let original_len = bytes.len();
    let Ok(decoded) = image::load_from_memory(&bytes) else {
        return img.clone();
    };

    // JPEG carries no alpha; manuscript scans are luminance-dominated.
    let mut rgb = decoded.to_rgb8();
    let mut out = encode_jpeg(&rgb, JPEG_QUALITY);
    let (mut w, mut h) = (rgb.width(), rgb.height());
    while out.len() > max_bytes && w.max(h) > min_long_side {
        let scale = ((max_bytes as f64 / out.len() as f64).sqrt() * 0.9).clamp(0.3, 0.9);
        let mut nw = ((w as f64 * scale) as u32).max(1);
        let mut nh = ((h as f64 * scale) as u32).max(1);
        // Never cross the legibility floor, even mid-iteration.
        if nw.max(nh) < min_long_side {
            let up = min_long_side as f64 / nw.max(nh) as f64;
            nw = ((nw as f64 * up) as u32).max(1);
            nh = ((nh as f64 * up) as u32).max(1);
        }
        if (nw, nh) == (w, h) {
            break;
        }
        (w, h) = (nw, nh);
        rgb = image::imageops::resize(&rgb, w, h, image::imageops::FilterType::Triangle);
        out = encode_jpeg(&rgb, JPEG_QUALITY);
    }
    // At the legibility floor but still over budget: trade JPEG quality, not
    // more pixels — q60 handwriting stays readable where a 600px shrink does not.
    let mut quality = JPEG_QUALITY;
    while out.len() > max_bytes && quality > 40 {
        quality -= 15;
        out = encode_jpeg(&rgb, quality);
    }
    warn!(
        before_bytes = original_len,
        after_bytes = out.len(),
        width = w,
        height = h,
        "SPEC-134: oversized vision image re-encoded (acquisition guard)"
    );
    ImageData {
        data: B64.encode(out),
        mime_type: "image/jpeg".into(),
        detail: img.detail.clone(),
    }
}

fn encode_jpeg(rgb: &image::RgbImage, quality: u8) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    match rgb.write_with_encoder(enc) {
        Ok(()) => buf.into_inner(),
        Err(_) => Vec::new(),
    }
}

#[async_trait]
impl LLMProvider for ImageGuardProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    fn max_context_length(&self) -> usize {
        self.inner.max_context_length()
    }

    async fn complete(&self, prompt: &str) -> Result<LLMResponse> {
        self.inner.complete(prompt).await
    }

    async fn complete_with_options(
        &self,
        prompt: &str,
        options: &CompletionOptions,
    ) -> Result<LLMResponse> {
        let options = with_effective_temperature(self.model(), options);
        self.inner.complete_with_options(prompt, &options).await
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let guarded = self.guard_messages_async(messages).await;
        let options = options.map(|o| with_effective_temperature(self.model(), o));
        self.inner.chat(&guarded, options.as_ref()).await
    }

    async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDefinition],
        tool_choice: Option<ToolChoice>,
        options: Option<&CompletionOptions>,
    ) -> Result<LLMResponse> {
        let guarded = self.guard_messages_async(messages).await;
        let options = options.map(|o| with_effective_temperature(self.model(), o));
        self.inner
            .chat_with_tools(&guarded, tools, tool_choice, options.as_ref())
            .await
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }

    fn supports_tool_streaming(&self) -> bool {
        self.inner.supports_tool_streaming()
    }

    fn supports_json_mode(&self) -> bool {
        self.inner.supports_json_mode()
    }

    fn supports_function_calling(&self) -> bool {
        self.inner.supports_function_calling()
    }

    async fn refresh_model_metadata(&self) -> Result<()> {
        self.inner.refresh_model_metadata().await
    }

    fn default_max_output_tokens(&self) -> Option<usize> {
        self.inner.default_max_output_tokens()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_temperature_for_models_that_only_accept_the_default() {
        let options = CompletionOptions {
            temperature: Some(0.1),
            max_tokens: Some(4096),
            ..Default::default()
        };
        let gated = with_effective_temperature("gpt-5.6-luna", &options);
        assert_eq!(gated.temperature, None);
        assert_eq!(gated.max_tokens, Some(4096));

        assert_eq!(
            with_effective_temperature("o4-mini", &options).temperature,
            None
        );
    }

    #[test]
    fn keeps_temperature_for_gpt_4_1_and_other_models() {
        let options = CompletionOptions {
            temperature: Some(0.1),
            ..Default::default()
        };
        for model in ["gpt-4.1-mini", "gpt-4.1-nano", "gemma4:latest"] {
            let kept = with_effective_temperature(model, &options);
            assert_eq!(kept.temperature, Some(0.1), "{model}");
        }
    }

    /// Build a PNG of the given dimensions filled with pseudo-random noise
    /// (noise compresses poorly → reliably exceeds the test budget).
    fn noisy_png(w: u32, h: u32) -> Vec<u8> {
        let mut img = image::RgbImage::new(w, h);
        for (x, y, px) in img.enumerate_pixels_mut() {
            let v = ((x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)) % 251) as u8;
            *px = image::Rgb([v, v, 255 - v]);
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        let enc = image::codecs::png::PngEncoder::new(&mut buf);
        img.write_with_encoder(enc).unwrap();
        buf.into_inner()
    }

    #[test]
    fn passthrough_when_under_budget() {
        let png = noisy_png(64, 64);
        let img = ImageData::new(B64.encode(&png), "image/png");
        let out = guard_image(&img, 1_000_000, PRINT_MIN_LONG_SIDE);
        assert_eq!(out.mime_type, "image/png");
        assert_eq!(out.data, img.data);
    }

    #[test]
    fn reencodes_to_jpeg_when_over_budget() {
        let png = noisy_png(1200, 900);
        // Budget between the JPEG q85 size and the PNG size: the re-encode
        // alone must fit, so dimensions are preserved.
        let jpeg_len = encode_jpeg(
            &image::load_from_memory(&png).unwrap().to_rgb8(),
            JPEG_QUALITY,
        )
        .len();
        let budget = jpeg_len + 10_000;
        assert!(png.len() > budget, "noise PNG must exceed the test budget");
        let img = ImageData::new(B64.encode(&png), "image/png");
        let out = guard_image(&img, budget, PRINT_MIN_LONG_SIDE);
        assert_eq!(out.mime_type, "image/jpeg");
        let bytes = B64.decode(&out.data).unwrap();
        assert!(
            bytes.len() <= budget,
            "guarded image must fit budget: {}",
            bytes.len()
        );
        // Dimensions preserved when JPEG re-encode alone fits the budget.
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert_eq!(decoded.width(), 1200);
        assert_eq!(decoded.height(), 900);
    }

    #[test]
    fn downscales_when_jpeg_alone_is_not_enough() {
        let png = noisy_png(3000, 2000);
        let img = ImageData::new(B64.encode(&png), "image/png");
        // Budget achievable only after downscaling: derive it from the
        // floor-sized JPEG so the test asserts the loop converges.
        let floor_rgb = image::imageops::resize(
            &image::load_from_memory(&png).unwrap().to_rgb8(),
            PRINT_MIN_LONG_SIDE,
            PRINT_MIN_LONG_SIDE * 2 / 3,
            image::imageops::FilterType::Triangle,
        );
        let budget = encode_jpeg(&floor_rgb, 40).len() + 5_000;
        let out = guard_image(&img, budget, PRINT_MIN_LONG_SIDE);
        let bytes = B64.decode(&out.data).unwrap();
        assert!(
            bytes.len() <= budget,
            "downscaled image must fit budget: {}",
            bytes.len()
        );
        let decoded = image::load_from_memory(&bytes).unwrap();
        assert!(decoded.width() < 3000, "must have downscaled");
        assert!(
            decoded.width().max(decoded.height()) >= PRINT_MIN_LONG_SIDE,
            "must not shrink below the legibility floor"
        );
    }

    #[test]
    fn manuscript_floor_does_not_shrink_below_2000() {
        let png = noisy_png(3600, 2400);
        let img = ImageData::new(B64.encode(&png), "image/png");
        let floor_rgb = image::imageops::resize(
            &image::load_from_memory(&png).unwrap().to_rgb8(),
            MANUSCRIPT_MIN_LONG_SIDE,
            MANUSCRIPT_MIN_LONG_SIDE * 2 / 3,
            image::imageops::FilterType::Triangle,
        );
        let budget = encode_jpeg(&floor_rgb, 40).len() + 8_000;
        let out = guard_image(&img, budget, MANUSCRIPT_MIN_LONG_SIDE);
        let decoded = image::load_from_memory(&B64.decode(&out.data).unwrap()).unwrap();
        assert!(
            decoded.width().max(decoded.height()) >= MANUSCRIPT_MIN_LONG_SIDE,
            "MS Pass-A must not crush below 2000px"
        );
    }

    #[test]
    fn urls_and_undecodable_pass_through() {
        let url_img = ImageData::from_url("https://example.com/x.png");
        let out = guard_image(&url_img, 1, PRINT_MIN_LONG_SIDE);
        assert!(out.is_url());
        let junk = ImageData::new(B64.encode(b"not an image"), "image/png");
        let out = guard_image(&junk, 1, PRINT_MIN_LONG_SIDE);
        assert_eq!(out.data, junk.data);
    }

    #[tokio::test]
    async fn chat_guards_images_end_to_end() {
        let png = noisy_png(1200, 900);
        let img = ImageData::new(B64.encode(&png), "image/png");
        let messages = vec![ChatMessage::user_with_images("describe", vec![img])];
        let mock = edgequake_llm::MockProvider::new();
        mock.add_response("ok").await;
        let budget = encode_jpeg(
            &image::load_from_memory(&png).unwrap().to_rgb8(),
            JPEG_QUALITY,
        )
        .len()
            + 10_000;
        let guarded = ImageGuardProvider {
            inner: Arc::new(mock),
            max_bytes: budget,
            min_long_side: PRINT_MIN_LONG_SIDE,
        };
        let out = guarded.guard_messages_async(&messages).await;
        let images = out[0].images.as_ref().unwrap();
        assert_eq!(images[0].mime_type, "image/jpeg");
        let bytes = B64.decode(&images[0].data).unwrap();
        assert!(bytes.len() <= budget);
    }
}
