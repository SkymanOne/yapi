// yapi: the generated catalogs read yapi's built-in catalog through the host,
// on first use, in place of pi's data files.
function catalog(type, provider) {
    let models;
    const load = () => (models ??= Object.fromEntries(globalThis.__yapi.request("models.list", { provider, type }).map((model) => [model.id, model])));
    return new Proxy({}, {
        get: (_target, key) => load()[key],
        has: (_target, key) => key in load(),
        ownKeys: () => Reflect.ownKeys(load()),
        getOwnPropertyDescriptor: (_target, key) => {
            const descriptor = Reflect.getOwnPropertyDescriptor(load(), key);
            if (descriptor)
                descriptor.configurable = true;
            return descriptor;
        },
    });
}
export function flattenChatModelCatalog(provider) {
    return catalog("chat", provider);
}
export function flattenImageModelCatalog(provider) {
    return catalog("image", provider);
}
export function flattenClassifierModelCatalog(provider) {
    return catalog("classifier", provider);
}
