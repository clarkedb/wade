#[path = "../scripts/build-info.rs"]
mod build_info;

fn main() {
    build_info::emit();
}
