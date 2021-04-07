mod win {
    #[allow(non_camel_case_types)]
    windows::include_bindings!();
    pub use Windows::ApplicationModel::Core::*;
    pub use Windows::Graphics::Display::*;
}

pub struct Monitor;

impl Monitor {
    pub fn current() {
        win::CoreApplication::Id();
        // let view = win::CoreApplication::MainView().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
        Monitor::current();
    }
}
