const path = require("path");

/** @type {import('electron-builder').Configuration} */
module.exports = {
  appId: "com.reelname.app",
  productName: "ReelName",
  npmRebuild: false,
  directories: {
    output: "dist-electron",
  },
  files: ["electron/**/*", "!node_modules"],
  extraResources: [
    {
      from: ".next/standalone",
      to: "standalone",
      filter: ["**/*"],
    },
    {
      from: ".next/standalone/node_modules",
      to: "standalone/node_modules",
      filter: ["**/*"],
    },
    {
      from: "build-resources/icon.png",
      to: "icon.png",
    },
    {
      from: "build-resources/${os}-node",
      to: "node",
    },
  ],
  win: {
    target: "nsis",
    icon: "build-resources/icon.ico",
    // Disable signing + icon editing (signing fails without admin symlink privilege).
    // We embed the icon manually via afterPack hook below.
    signAndEditExecutable: false,
  },
  nsis: {
    oneClick: false,
    allowToChangeInstallationDirectory: true,
    allowElevation: true,
  },
  mac: {
    target: "dmg",
    icon: "build-resources/icon-1024.png",
    category: "public.app-category.utilities",
  },
  linux: {
    target: ["AppImage", "deb"],
    icon: "build-resources/icon.png",
    category: "Utility",
    maintainer: "ReelName <reelname@reelname.app>",
  },
  afterPack: async (context) => {
    // Manually set the exe icon via rcedit since signAndEditExecutable is false.
    // This avoids the winCodeSign download which fails due to macOS symlinks.
    if (context.electronPlatformName !== "win32") return;

    const exePath = path.join(
      context.appOutDir,
      `${context.packager.appInfo.productFilename}.exe`
    );
    const icoPath = path.resolve(__dirname, "build-resources", "icon.ico");

    const { rcedit } = require("rcedit");
    await rcedit(exePath, {
      icon: icoPath,
      "version-string": {
        ProductName: "ReelName",
        FileDescription: "ReelName",
        CompanyName: "ReelName",
      },
    });
    console.log("  Custom icon embedded via rcedit");
  },
};
