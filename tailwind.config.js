module.exports = {
  future: {
    removeDeprecatedGapUtilities: true,
  },
  purge: [
    './pasta6_core/templates/base.html',
    './pasta6_home/templates/index.html',
    './pasta6_meta/templates/index.html',
    './pasta6_meta/templates/login.html',
    './pasta6_meta/templates/profile.html',
    './pasta6_meta/templates/register.html',
    './pasta6_paste/templates/index.html',
    './pasta6_paste/templates/paste.html',
  ],
  plugins: [
    require('tailwindcss'),
    require('autoprefixer'),
  ],
}
