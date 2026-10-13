// Loaded as a blocking script so a saved theme applies before the first paint.
// Without a saved choice, the page follows the system color scheme.
{
  const root = document.documentElement;
  const systemLight = window.matchMedia("(prefers-color-scheme: light)");

  try {
    const saved = localStorage.getItem("wade-theme");
    if (saved === "light" || saved === "dark") root.dataset.theme = saved;
  } catch {}

  const isLight = () =>
    root.dataset.theme ? root.dataset.theme === "light" : systemLight.matches;

  document.addEventListener("DOMContentLoaded", () => {
    const toggle = document.querySelector("#theme-toggle");
    const label = () => {
      toggle.textContent = isLight() ? "Dark mode" : "Light mode";
    };
    toggle.addEventListener("click", () => {
      root.dataset.theme = isLight() ? "dark" : "light";
      try {
        localStorage.setItem("wade-theme", root.dataset.theme);
      } catch {}
      label();
    });
    systemLight.addEventListener("change", label);
    label();
    toggle.hidden = false;
  });
}
