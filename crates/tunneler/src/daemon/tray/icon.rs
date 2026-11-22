//! The tray icons, drawn procedurally: the Tunneler logo (a dark rounded
//! square holding an arch with a dark opening), whose arch colour carries the
//! global status. The status is also text in the tooltip and the menu, so the
//! colour is never the only carrier.

use super::view::GlobalStatus;

const ICON_SIZE: u32 = 32;
const SUBSAMPLES_PER_AXIS: u32 = 4;
const CENTER: f32 = 16.0;
const BACKGROUND_CORNER_RADIUS: f32 = 7.0;
const ARCH_BOTTOM: f32 = 26.0;
const BACKGROUND_COLOR: [u8; 3] = [31, 41, 55];
const OPENING_COLOR: [u8; 3] = [15, 23, 42];
const IDLE_COLOR: [u8; 3] = [158, 158, 158];
const HEALTHY_COLOR: [u8; 3] = [46, 160, 67];
const DEGRADED_COLOR: [u8; 3] = [245, 166, 35];
const ERROR_COLOR: [u8; 3] = [220, 53, 69];
const ARCH: ArchShape = ArchShape {
    radius: 10.0,
    circle_center_y: 15.0,
};
const OPENING: ArchShape = ArchShape {
    radius: 5.0,
    circle_center_y: 15.5,
};

/// A half circle on top of a box that stops at [`ARCH_BOTTOM`].
#[derive(Clone, Copy)]
struct ArchShape {
    radius: f32,
    circle_center_y: f32,
}

/// A square RGBA picture, row by row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IconImage {
    /// Four bytes per pixel: red, green, blue, alpha.
    pub(crate) rgba: Vec<u8>,
    /// Pixels per row.
    pub(crate) width: u32,
    /// Number of rows.
    pub(crate) height: u32,
}

fn distance_to_rounded_square(x: f32, y: f32) -> f32 {
    let inner_half_size = CENTER - BACKGROUND_CORNER_RADIUS;
    let offset_x = (x - CENTER).abs() - inner_half_size;
    let offset_y = (y - CENTER).abs() - inner_half_size;
    offset_x.max(0.0).hypot(offset_y.max(0.0)) + offset_x.max(offset_y).min(0.0)
        - BACKGROUND_CORNER_RADIUS
}

fn distance_to_arch(shape: ArchShape, x: f32, y: f32) -> f32 {
    let offset_x = (x - CENTER).abs();
    if y < shape.circle_center_y {
        return offset_x.hypot(y - shape.circle_center_y) - shape.radius;
    }
    let outside_x = offset_x - shape.radius;
    let outside_y = y - ARCH_BOTTOM;
    outside_x.max(0.0).hypot(outside_y.max(0.0)) + outside_x.max(outside_y).min(0.0)
}

fn arch_color(status: GlobalStatus) -> [u8; 3] {
    match status {
        GlobalStatus::Idle => IDLE_COLOR,
        GlobalStatus::Healthy => HEALTHY_COLOR,
        GlobalStatus::Degraded => DEGRADED_COLOR,
        GlobalStatus::Error => ERROR_COLOR,
    }
}

fn sample_color(status: GlobalStatus, x: f32, y: f32) -> Option<[u8; 3]> {
    if distance_to_arch(OPENING, x, y) <= 0.0 {
        return Some(OPENING_COLOR);
    }
    if distance_to_arch(ARCH, x, y) <= 0.0 {
        return Some(arch_color(status));
    }
    if distance_to_rounded_square(x, y) <= 0.0 {
        return Some(BACKGROUND_COLOR);
    }
    None
}

#[allow(clippy::cast_precision_loss)]
fn pixel(status: GlobalStatus, column: u32, row: u32) -> [u8; 4] {
    let step = 1.0 / SUBSAMPLES_PER_AXIS as f32;
    let mut covered_samples = 0_u32;
    let mut color_sum = [0_u32; 3];
    for sub_row in 0..SUBSAMPLES_PER_AXIS {
        for sub_column in 0..SUBSAMPLES_PER_AXIS {
            let x = column as f32 + (sub_column as f32 + 0.5) * step;
            let y = row as f32 + (sub_row as f32 + 0.5) * step;
            let Some(color) = sample_color(status, x, y) else {
                continue;
            };
            covered_samples += 1;
            for (sum, channel) in color_sum.iter_mut().zip(color) {
                *sum += u32::from(channel);
            }
        }
    }
    blend(covered_samples, color_sum)
}

fn blend(covered_samples: u32, color_sum: [u32; 3]) -> [u8; 4] {
    if covered_samples == 0 {
        return [0, 0, 0, 0];
    }
    let total_samples = SUBSAMPLES_PER_AXIS * SUBSAMPLES_PER_AXIS;
    let average = |sum: u32| u8::try_from(sum / covered_samples).unwrap_or(u8::MAX);
    let alpha =
        u8::try_from(covered_samples * u32::from(u8::MAX) / total_samples).unwrap_or(u8::MAX);
    [
        average(color_sum[0]),
        average(color_sum[1]),
        average(color_sum[2]),
        alpha,
    ]
}

/// The icon of a status: the logo as a 32×32 anti-aliased picture.
pub(crate) fn render_icon(status: GlobalStatus) -> IconImage {
    let rgba = (0..ICON_SIZE)
        .flat_map(|row| (0..ICON_SIZE).map(move |column| (column, row)))
        .flat_map(|(column, row)| pixel(status, column, row))
        .collect();
    IconImage {
        rgba,
        width: ICON_SIZE,
        height: ICON_SIZE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_STATUSES: [GlobalStatus; 4] = [
        GlobalStatus::Idle,
        GlobalStatus::Healthy,
        GlobalStatus::Degraded,
        GlobalStatus::Error,
    ];
    const HUE_TOLERANCE_DEGREES: f32 = 12.0;
    const ARCH_SIDE_PIXEL: (u32, u32) = (8, 22);
    const OPENING_PIXEL: (u32, u32) = (16, 22);
    const BACKGROUND_PIXEL: (u32, u32) = (3, 16);

    fn pixel_at(status: GlobalStatus, (column, row): (u32, u32)) -> [u8; 4] {
        let offset = ((row * ICON_SIZE + column) * 4) as usize;
        let rgba = render_icon(status).rgba;
        [
            rgba[offset],
            rgba[offset + 1],
            rgba[offset + 2],
            rgba[offset + 3],
        ]
    }

    fn color_at(status: GlobalStatus, position: (u32, u32)) -> [u8; 3] {
        let [red, green, blue, _] = pixel_at(status, position);
        [red, green, blue]
    }

    fn alpha_channel(status: GlobalStatus) -> Vec<u8> {
        render_icon(status)
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[3])
            .collect()
    }

    fn hue_degrees([red, green, blue]: [u8; 3]) -> f32 {
        let maximum = red.max(green).max(blue);
        let minimum = red.min(green).min(blue);
        if maximum == minimum {
            return 0.0;
        }
        let delta = f32::from(maximum - minimum);
        let gap = |first: u8, second: u8| (f32::from(first) - f32::from(second)) / delta;
        let sector = if maximum == red {
            gap(green, blue).rem_euclid(6.0)
        } else if maximum == green {
            gap(blue, red) + 2.0
        } else {
            gap(red, green) + 4.0
        };
        sector * 60.0
    }

    fn assert_hue_near(status: GlobalStatus, expected_degrees: f32) {
        let hue = hue_degrees(color_at(status, ARCH_SIDE_PIXEL));
        let gap = (hue - expected_degrees).abs();
        let circular_gap = gap.min(360.0 - gap);
        assert!(
            circular_gap <= HUE_TOLERANCE_DEGREES,
            "{status:?}: hue {hue} is not near {expected_degrees}"
        );
    }

    #[test]
    fn should_fill_a_buffer_of_width_times_height_times_four_bytes() {
        for status in ALL_STATUSES {
            let icon = render_icon(status);

            assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        }
    }

    #[test]
    fn should_draw_a_square_icon_of_32_pixels() {
        let icon = render_icon(GlobalStatus::Idle);

        assert_eq!((icon.width, icon.height), (ICON_SIZE, ICON_SIZE));
    }

    #[test]
    fn should_draw_the_same_silhouette_for_every_status() {
        let reference = alpha_channel(GlobalStatus::Idle);

        for status in ALL_STATUSES {
            assert_eq!(alpha_channel(status), reference, "{status:?}");
        }
    }

    #[test]
    fn should_leave_the_corners_transparent() {
        for status in ALL_STATUSES {
            assert_eq!(render_icon(status).rgba[3], 0, "{status:?}");
        }
    }

    #[test]
    fn should_use_a_grey_arch_when_idle() {
        let [red, green, blue] = color_at(GlobalStatus::Idle, ARCH_SIDE_PIXEL);

        assert!(red == green && green == blue);
    }

    #[test]
    fn should_use_a_green_arch_when_healthy() {
        assert_hue_near(GlobalStatus::Healthy, 130.0);
    }

    #[test]
    fn should_use_an_amber_arch_when_degraded() {
        assert_hue_near(GlobalStatus::Degraded, 38.0);
    }

    #[test]
    fn should_use_a_red_arch_when_in_error() {
        assert_hue_near(GlobalStatus::Error, 353.0);
    }

    #[test]
    fn should_keep_the_opening_the_same_color_for_every_status() {
        for status in ALL_STATUSES {
            assert_eq!(color_at(status, OPENING_PIXEL), OPENING_COLOR, "{status:?}");
        }
    }

    #[test]
    fn should_keep_the_background_the_same_color_for_every_status() {
        for status in ALL_STATUSES {
            assert_eq!(
                color_at(status, BACKGROUND_PIXEL),
                BACKGROUND_COLOR,
                "{status:?}"
            );
        }
    }
}
