fn main() {
    windows::build!(
        Windows::Graphics::Display::{DisplayInformation, AdvancedColorInfo, DisplayOrientations},
        Windows::ApplicationModel::Core::CoreApplication
    )
}
