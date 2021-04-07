use monitor_control_win::Monitor;

fn main() {
    let dev = Monitor::list().unwrap().first().unwrap().display_device();
    println!("{:#?}", dev);
}
