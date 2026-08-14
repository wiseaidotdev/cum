/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./index.html", "./src/**/*.rs"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
      },
      colors: {
        um: {
          bg: "#141414",
          surface: "#1c1c1c",
          elevated: "#242424",
          border: "#2e2e2e",
          text: "#e8e8e8",
          muted: "#888888",
          subtle: "#555555",
          accent: "#7c3aed",
          "accent-hover": "#6d28d9",
        },
      },
    },
  },
  plugins: [],
};
