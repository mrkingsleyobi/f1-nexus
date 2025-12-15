// F1 Nexus npm package entry point
// This package provides the f1-nexus CLI tool

module.exports = {
  version: require('./package.json').version,
  bin: require.resolve('./bin/f1-nexus')
};
