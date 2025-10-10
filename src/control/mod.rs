pub fn start_charging() {
    info!("starting charging");
}

pub fn stop_charging() {
    info!("stopping charging");
}

pub fn inc_current(step: u32) {
    info!("increasing current by {} A", (step as f32) * 0.1);
}

pub fn dec_current(step: u32) {
    info!("decreasing current by {} A", (step as f32) * 0.1);
}
