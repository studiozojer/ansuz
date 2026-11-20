export const abi = 1;
export const roster = {};

let loading;
function loadEngine() {
  if (!loading) {
    // Exact serves this page module at modules/index.js. The bundled Wasm
    // loader finds its binary beside itself even when hosted at a subpath.
    const path = new URL('../assets/jianyi/jianyi.js', import.meta.url);
    loading = import(path.href).then(async engine => {
      await engine.default();
      return engine;
    }).catch(error => {
      loading = undefined;
      throw error;
    });
  }
  return loading;
}

export async function later(request) {
  if (request.op !== 'cast' || typeof request.seed !== 'string') {
    throw new Error('Unsupported Jianyi request');
  }
  const engine = await loadEngine();
  return JSON.parse(engine.cast(JSON.stringify({ timestamp: request.seed })));
}
