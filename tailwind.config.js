module.exports = {
  future: {
    removeDeprecatedGapUtilities: true,
  },
  purge: [
    './templates/base.html',
    './templates/index.html',
    './templates/paste.html',
  ],
  plugins: [
    require('tailwindcss'),
    require('autoprefixer'),
  ],
}
