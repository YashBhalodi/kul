// The landing page's build-time content (src/landing/vite-plugin.ts).
declare module "virtual:kul-landing" {
  const content: import("./landing-data").LandingContent
  export default content
}
