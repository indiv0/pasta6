module.exports = {
  future: {
    removeDeprecatedGapUtilities: true,
  },
  purge: [
    './templates/index.html',
    './templates/paste.html',
  ],
  plugins: [
    require('tailwindcss'),
    require('autoprefixer'),
  ],
}
