module.exports = {
  future: {
    removeDeprecatedGapUtilities: true,
  },
  purge: [
    './templates/base.html',
    './templates/index.html',
    './templates/paste.html',
    './templates/register.html',
  ],
  plugins: [
    require('tailwindcss'),
    require('autoprefixer'),
  ],
}
