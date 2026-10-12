//! Lines and markers on one set of axes: `cargo run --example lines`.

use rapl::*;
use rapl_plot::*;

fn main() -> Result<(), rapl_plot::Error> {
    let t = Ndarr::from_fn([240], |ix| ix[0] as f64 * 0.025);

    // [240, 3]: one damped oscillation per column, all against the same `t`.
    let decay = Ndarr::from([0.3, 0.8, 1.6]);
    let waves = Ndarr::from_fn([240, 3], |ix| {
        let t = t[[ix[0]]];
        (-decay[[ix[1]]] * t).exp() * (std::f64::consts::TAU * t).cos()
    });

    // A few noisy samples of the slowest one.
    let sampled_t = t.slice(s![Slice::stepped(.., 12)])?.to_owned_array();
    let samples = sampled_t
        .map(|&t| (-0.3 * t).exp() * (std::f64::consts::TAU * t).cos() + 0.06 * (t * 37.0).sin());

    let plot = (line(&t, &waves)
        .label(decay.map(|d| format!("decay {d} /s")))
        .width([2.0, 1.5, 1.5])
        + scatter(&sampled_t, &samples)
            .color(Color::BLACK)
            .size(5)
            .label("samples"))
    .title("Damped oscillations")
    .x(axis().label("time (s)"))
    .y(axis().label("amplitude").lim(-1.1, 1.1));

    plot.save("lines.svg")?;
    println!("wrote lines.svg");
    Ok(())
}
