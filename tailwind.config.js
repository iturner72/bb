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
        'ucla-blue': {
          DEFAULT: '#2375A7',
          100: '#071722',
          200: '#0e2f43',
          300: '#154665',
          400: '#1d5e87',
          500: '#2375a7',
          600: '#3597d4',
          700: '#68b1df',
          800: '#9acbea',
          900: '#cde5f4'
        },
        'dark-purple': {
          DEFAULT: '#401830',
          100: '#0d0509',
          200: '#190a13',
          300: '#260e1c',
          400: '#321326',
          500: '#401830',
          600: '#7c2f5d',
          700: '#b9478b',
          800: '#d084b2',
          900: '#e8c2d8'
        },
        'celestial-blue': {
          DEFAULT: '#4E97D1',
          100: '#0c1e2d',
          200: '#173c5b',
          300: '#235b88',
          400: '#2f79b6',
          500: '#4e97d1',
          600: '#70abda',
          700: '#94c0e3',
          800: '#b8d5ed',
          900: '#dbeaf6'
        },
        'tyrian-purple': {
          DEFAULT: '#682146',
          100: '#15070e',
          200: '#2a0d1c',
          300: '#3f142a',
          400: '#541a38',
          500: '#682146',
          600: '#a1336c',
          700: '#c95591',
          800: '#db8eb6',
          900: '#edc6da'
        },
        'rich-black': {
          DEFAULT: '#0F0D19',
          100: '#030205',
          200: '#060509',
          300: '#08070e',
          400: '#0b0a13',
          500: '#0f0d19',
          600: '#332c56',
          700: '#594d95',
          800: '#8c82be',
          900: '#c5c0df'
        }
      },
      extend: {
          height: {
              '108': '26rem',
              '128': '32rem',
              '172': '64rem',
          },
          backgroundImage: {
            'gradient-top': 'linear-gradient(0deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-right': 'linear-gradient(90deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-bottom': 'linear-gradient(180deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-left': 'linear-gradient(270deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-top-right': 'linear-gradient(45deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-bottom-right': 'linear-gradient(135deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-top-left': 'linear-gradient(225deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-bottom-left': 'linear-gradient(315deg, #0F0D19, #401830, #2375A7, #4E97D1, #682146)',
            'gradient-radial': 'radial-gradient(#0F0D19, #401830, #2375A7, #4E97D1, #682146)',
          },
      },
    },
    plugins: [],
}
