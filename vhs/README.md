# Demo Gif Creation

This folder contains files to create a pasejo usage demo using the [VHS project](https://github.com/charmbracelet/vhs).

1. Create container image: `podman build --tag vhs:pasejo --file vhs/Containerfile .`
2. Run the container: `podman run --rm -v $PWD/vhs:/vhs vhs:pasejo demo.tape`
3. The demo will be created in the `vhs` folder as `demo.gif`

Modify the [demo.tape](./demo.tape) file to change the demo content. The VHS project uses a simple DSL to create the demo. You can find more information about it in the [VHS documentation](https://github.com/charmbracelet/vhs?tab=readme-ov-file#vhs-command-reference).
