// SPDX-FileCopyrightText: The pasejo Authors
// SPDX-License-Identifier: 0BSD

//! Generates the Kotlin and Swift bindings from the built library.

fn main() {
    uniffi::uniffi_bindgen_main();
}
