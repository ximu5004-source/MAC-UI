/// Keep context generation isolated, but never serve a partially built frontend.
pub fn get_context() -> tauri::Context {
    let context = tauri::generate_context!("tauri.conf.json");
    #[cfg(feature = "custom-protocol")]
    for path in [
        "react/settings/index.html",
        "svelte/weg/index.html",
        "svelte/fancy-toolbar/index.html",
        "svelte/flyouts/index.html",
        "svelte/apps-menu/index.html",
        "svelte/task-switcher/index.html",
    ] {
        assert!(
            context.assets().get(&path.into()).is_some(),
            "Production frontend was not embedded: {path}; rebuild after the frontend build finishes"
        );
    }
    context
}

#[cfg(all(test, feature = "custom-protocol"))]
mod tests {
    #[test]
    fn standalone_binary_contains_every_required_frontend() {
        super::get_context();
    }
}
