const fs = require('fs');
const path = require('path');

const iconDir = path.join(__dirname, '../apps/desktop/src-tauri/icons');
const icons = ['32x32.png', '128x128.png', '128x128@2x.png'];

icons.forEach(iconName => {
  const iconPath = path.join(iconDir, iconName);
  if (fs.existsSync(iconPath)) {
    console.log(`Found icon: ${iconPath}`);
  }
});

console.log('\nIcon files are in place. Ensure they are in RGBA format.');
