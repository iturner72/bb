/** @type {import('tailwindcss').Config} */
/*
    * run this 
    * npx tailwindcss -i ./style/tailwind.css -o ./style/output.css --watch
    */
module.exports = {
  content: {
    files: ["*.html", "./src/**/*.rs"],
  },
  darkMode: "class",
  theme: {
    colors: {
      transparent: 'transparent',
      current: 'currentColor',
      'white': '#ffffff',
      'black': '#000000',
      
      // Use CSS custom properties - these will be defined in your CSS
      'gray': {
        DEFAULT: 'var(--color-gray)',
        900: 'var(--color-gray-900)',
        800: 'var(--color-gray-800)',
        700: 'var(--color-gray-700)',
        600: 'var(--color-gray-600)',
        500: 'var(--color-gray-500)',
        400: 'var(--color-gray-400)',
        300: 'var(--color-gray-300)',
        200: 'var(--color-gray-200)',
        100: 'var(--color-gray-100)'
      },
      'teal': {
        DEFAULT: 'var(--color-teal)',
        900: 'var(--color-teal-900)',
        800: 'var(--color-teal-800)',
        700: 'var(--color-teal-700)',
        600: 'var(--color-teal-600)',
        500: 'var(--color-teal-500)',
        400: 'var(--color-teal-400)',
        300: 'var(--color-teal-300)',
        200: 'var(--color-teal-200)',
        100: 'var(--color-teal-100)'
      },
      'mint': {
        DEFAULT: 'var(--color-mint)',
        900: 'var(--color-mint-900)',
        800: 'var(--color-mint-800)',
        700: 'var(--color-mint-700)',
        600: 'var(--color-mint-600)',
        500: 'var(--color-mint-500)',
        400: 'var(--color-mint-400)',
        300: 'var(--color-mint-300)',
        200: 'var(--color-mint-200)',
        100: 'var(--color-mint-100)'
      },
      'seafoam': {
        DEFAULT: 'var(--color-seafoam)',
        900: 'var(--color-seafoam-900)',
        800: 'var(--color-seafoam-800)',
        700: 'var(--color-seafoam-700)',
        600: 'var(--color-seafoam-600)',
        500: 'var(--color-seafoam-500)',
        400: 'var(--color-seafoam-400)',
        300: 'var(--color-seafoam-300)',
        200: 'var(--color-seafoam-200)',
        100: 'var(--color-seafoam-100)'
      },
      'wenge': {
        DEFAULT: 'var(--color-wenge)',
        900: 'var(--color-wenge-900)',
        800: 'var(--color-wenge-800)',
        700: 'var(--color-wenge-700)',
        600: 'var(--color-wenge-600)',
        500: 'var(--color-wenge-500)',
        400: 'var(--color-wenge-400)',
        300: 'var(--color-wenge-300)',
        200: 'var(--color-wenge-200)',
        100: 'var(--color-wenge-100)'
      },
      'aqua': {
        DEFAULT: 'var(--color-aqua)',
        900: 'var(--color-aqua-900)',
        800: 'var(--color-aqua-800)',
        700: 'var(--color-aqua-700)',
        600: 'var(--color-aqua-600)',
        500: 'var(--color-aqua-500)',
        400: 'var(--color-aqua-400)',
        300: 'var(--color-aqua-300)',
        200: 'var(--color-aqua-200)',
        100: 'var(--color-aqua-100)'
      },
      'salmon': {
        DEFAULT: 'var(--color-salmon)',
        900: 'var(--color-salmon-900)',
        800: 'var(--color-salmon-800)',
        700: 'var(--color-salmon-700)',
        600: 'var(--color-salmon-600)',
        500: 'var(--color-salmon-500)',
        400: 'var(--color-salmon-400)',
        300: 'var(--color-salmon-300)',
        200: 'var(--color-salmon-200)',
        100: 'var(--color-salmon-100)'
      },
      'aquamarine': {
        DEFAULT: 'var(--color-aquamarine)',
        light: 'var(--color-aquamarine-light)',
        dark: 'var(--color-aquamarine-dark)'
      },
      'purple': {
        DEFAULT: 'var(--color-purple)',
        light: 'var(--color-purple-light)',
        dark: 'var(--color-purple-dark)'
      },
      'orange': {
        DEFAULT: 'var(--color-orange)',
        light: 'var(--color-orange-light)',
        dark: 'var(--color-orange-dark)'
      },
    },
    extend: {
      height: {
        '108': '26rem',
        '128': '32rem',
        '172': '64rem',
      },
    boxShadow: {
      'seafoam-light': '0 4px 6px -1px rgba(134, 239, 172, 0.08), 0 2px 4px -2px rgba(134, 239, 172, 0.08)',
      'mint-glow': '0 4px 6px -1px rgba(167, 243, 208, 0.12), 0 2px 4px -2px rgba(167, 243, 208, 0.1)',
      'aqua-soft': '0 4px 6px -1px rgba(103, 232, 249, 0.1), 0 2px 4px -2px rgba(103, 232, 249, 0.08)',
      'teal-highlight': '0 6px 8px -2px rgba(20, 184, 166, 0.15), 0 2px 4px -1px rgba(20, 184, 166, 0.1)',
      'white-ethereal': '0 4px 6px -1px rgba(255, 255, 255, 0.06), 0 2px 4px -2px rgba(255, 255, 255, 0.04)'
    }
    },
  },
  plugins: [],
}
