pub fn recipient_added() {
    println!("Recipient added");
}

pub fn recipient_updated() {
    println!("Recipient updated");
}

pub fn identity_added() {
    println!("Identity added");
}

pub fn identity_removed() {
    println!("Identity removed");
}

pub fn store_initialized(store_path: &str) {
    println!("Store initialized at {store_path}");
}

pub fn store_set_default(store_name: &str) {
    println!("Store {store_name} is now the default");
}

pub fn store_removed(store_name: &str) {
    println!("Store {store_name} removed");
}
