resource "google_storage_bucket" "artifacts" {
  name                        = "${var.project_id}-${var.name_prefix}-artifacts"
  location                    = var.region
  project                     = var.project_id
  uniform_bucket_level_access = true
  public_access_prevention    = "enforced"
  force_destroy               = false
  labels                      = local.labels

  depends_on = [google_project_service.required]
}

# Document bytes for the demo. The JSON key is not in Terraform state.
# Import before the next apply:
#   terraform import google_storage_bucket.originals saas-app-001-edgequake-originals
resource "google_storage_bucket" "originals" {
  name                        = "${var.project_id}-${var.name_prefix}-originals"
  location                    = var.region
  project                     = var.project_id
  uniform_bucket_level_access = true
  public_access_prevention    = "enforced"
  force_destroy               = false
  labels                      = local.labels

  depends_on = [google_project_service.required]
}

resource "google_storage_bucket_object" "artifacts" {
  for_each = local.artifact_files
  bucket   = google_storage_bucket.artifacts.name
  name     = each.key
  source   = each.value
}
