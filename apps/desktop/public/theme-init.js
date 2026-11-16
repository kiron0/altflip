(function () {
  var theme = "system";
  try {
    theme = localStorage.getItem("altflip.theme") || "system";
  } catch (e) {}
  var dark = theme === "dark" || (theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  var root = document.documentElement;
  root.dataset.theme = dark ? "dark" : "light";
  root.style.colorScheme = dark ? "dark" : "light";
  var m = location.search.match(/[?&]surface=([^&]+)/);
  root.dataset.surface = m ? m[1] : "main";
})();
