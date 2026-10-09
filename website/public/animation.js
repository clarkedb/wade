const wade = document.querySelector("#wade");
const control = document.querySelector("#animation-control");
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
let playing = !reducedMotion.matches;

function render() {
  wade.src = playing ? "wade.gif" : "wade.png";
  const label = playing ? "Pause Wade's animation" : "Play Wade's animation";
  control.setAttribute("aria-label", label);
  control.title = label;
}

control.addEventListener("click", () => {
  playing = !playing;
  render();
});
reducedMotion.addEventListener("change", () => {
  if (reducedMotion.matches) {
    playing = false;
    render();
  }
});

render();
control.disabled = false;
