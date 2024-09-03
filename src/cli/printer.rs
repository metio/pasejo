pub fn recipient_added() {
    println!("Recipient added");
}

pub fn recipient_updated() {
    println!("Recipient updated");
}

pub fn identity_added() {
    println!("Identity added");
}

pub fn store_initialized(store_path: String) {
    println!("Store initialized at {}", store_path);
}

pub fn store_set_default(store_alias: String) {
    println!("Store {} is now the default", store_alias);
}
