locals {
  labels = {
    spec                       = "148"
    component                  = "edgequake"
    goog-terraform-provisioned = "true"
  }

  public_origin = var.hostname != "" ? "https://${var.hostname}" : "https://${google_compute_address.ip.address}"

  required_services = toset([
    "compute.googleapis.com",
    "iam.googleapis.com",
    "iamcredentials.googleapis.com",
    "iap.googleapis.com",
    "secretmanager.googleapis.com",
    "storage.googleapis.com",
    "sts.googleapis.com",
    "logging.googleapis.com",
    "monitoring.googleapis.com",
    "oslogin.googleapis.com",
  ])

  artifact_files = {
    "compose/docker-compose.yml" = "${path.module}/../compose/docker-compose.yml"
    "compose/Caddyfile"          = "${path.module}/../compose/Caddyfile"
    "compose/Caddyfile.hostname" = "${path.module}/../compose/Caddyfile.hostname"
    "compose/snippets.caddy"     = "${path.module}/../compose/snippets.caddy"
    "compose/.env.example"       = "${path.module}/../compose/.env.example"
    "scripts/deploy.sh"          = "${path.module}/../scripts/deploy.sh"
    "scripts/install-release.sh" = "${path.module}/../scripts/install-release.sh"
    "scripts/render-env.sh"       = "${path.module}/../scripts/render-env.sh"
    "scripts/render-originals.sh" = "${path.module}/../scripts/render-originals.sh"
    "scripts/load-metadata.sh"   = "${path.module}/../scripts/load-metadata.sh"
    "scripts/generate-tls.sh"    = "${path.module}/../scripts/generate-tls.sh"
  }
}
