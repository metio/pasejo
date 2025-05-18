# Demo Gif Creation

This folder contains files to create a pasejo usage demo using the [VHS project](https://github.com/charmbracelet/vhs).

1. Build a release version of pasejo: `cargo build --release`
2. Create container image: `podman build --tag vhs:pasejo --file vhs/Containerfile .`
3. Run the container: `podman run --rm -v $PWD/vhs:/vhs vhs:pasejo demo.tape`
4. The demo will be created in the `vhs` folder as `demo.gif`

Modify the [demo.tape](./demo.tape) file to change the demo content. The VHS project uses a simple DSL to create the demo. You can find more information about it in the [VHS documentation](https://github.com/charmbracelet/vhs?tab=readme-ov-file#vhs-command-reference).
