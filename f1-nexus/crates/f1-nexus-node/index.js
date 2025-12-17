const { platform, arch } = process;

let nativeBinding = null;

const loadError = new Error('Failed to load native binding');

function loadNative() {
  const suffix = `${platform}-${arch}`;
  
  try {
    nativeBinding = require(`./f1-nexus-node.${suffix}.node`);
  } catch (e) {
    loadError.message = `Failed to load native binding for ${suffix}. Error: ${e.message}`;
    throw loadError;
  }
}

loadNative();

module.exports = nativeBinding;
