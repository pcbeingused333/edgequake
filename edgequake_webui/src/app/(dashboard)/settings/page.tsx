import { redirect } from "next/navigation";

/**
 * `/settings` has no section of its own. Land on Appearance, which every
 * caller can open. The shell then renders one h1 for that section.
 */
export default function SettingsIndexPage() {
  redirect("/settings/you/appearance");
}
