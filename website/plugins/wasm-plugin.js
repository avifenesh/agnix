// @ts-check

/** Docusaurus plugin to enable WASM file handling in webpack. */
module.exports = function wasmPlugin() {
  return {
    name: 'wasm-plugin',
    configureWebpack() {
      return {
        experiments: {
          asyncWebAssembly: true,
        },
      };
    },
  };
};
