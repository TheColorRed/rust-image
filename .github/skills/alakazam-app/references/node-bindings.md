# Bindings

## Library integration/bindings

The Alakazam App uses the Abra library as its backend for image manipulation. The bindings for the Abra library are generated using `napi-rs` and are located in `/packages/node/alakazam`. The bindings are build into a file called `abra.node`, which is then used by the Electron app to perform image manipulation tasks.

### Building the bindings

To build the bindings, run the following command from the root of the repository, pick the most appropriate one, as building all can take a longer time than just building one of them:

```bash
# Builds the Alakazam bindings
npm run build-alakazam

# Builds the Gizmos bindings
npm run build-gizmos

# Builds all bindings
npm run build-bindings
```
