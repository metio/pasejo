pub fn recipient_added() {
    println!("Recipient added");
}

pub fn recipient_updated() {
    println!("Recipient updated");
}

pub fn identity_added() {
    println!("Identity added");
}

pub fn store_initialized(store_path: &str) {
    println!("Store initialized at {store_path}");
}

pub fn store_set_default(store_alias: &str) {
    println!("Store {store_alias} is now the default");
}

pub fn store_removed(store_alias: &str) {
    println!("Store {store_alias} removed");
}
