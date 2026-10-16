//! The tray icons, drawn procedurally: one distinct shape per status, so that
//! the status never relies on colour alone, with a dark outline that reads on
//! light and dark panels.

use super::view::GlobalStatus;

const ICON_SIZE: u32 = 32;
const SUBSAMPLES_PER_AXIS: u32 = 4;
const OUTLINE_WIDTH: f32 = 1.5;
const CENTER: f32 = 16.0;
const RING_RADIUS: f32 = 11.0;
const RING_HALF_THICKNESS: f32 = 4.0;
const DISC_RADIUS: f32 = 13.5;
const OUTLINE_COLOR: [u8; 3] = [30, 30, 30];
const IDLE_COLOR: [u8; 3] = [158, 158, 158];
const HEALTHY_COLOR: [u8; 3] = [46, 160, 67];
const DEGRADED_COLOR: [u8; 3] = [245, 166, 35];
const ERROR_COLOR: [u8; 3] = [220, 53, 69];
const TRIANGLE: [(f32, f32); 3] = [(16.0, 2.5), (29.5, 27.5), (2.5, 27.5)];
const DIAMOND: [(f32, f32); 4] = [(16.0, 2.0), (30.0, 16.0), (16.0, 30.0), (2.0, 16.0)];

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

/// Where a sample lies relative to the shape.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Sample {
    Outside,
    Outline,
    Fill,
}

fn distance_to_circle(x: f32, y: f32, radius: f32) -> f32 {
    (x - CENTER).hypot(y - CENTER) - radius
}

fn distance_to_ring(x: f32, y: f32) -> f32 {
    (distance_to_circle(x, y, RING_RADIUS)).abs() - RING_HALF_THICKNESS
}

/// The largest signed distance to the lines of a convex polygon whose
/// vertices are in the orientation of [`TRIANGLE`] and [`DIAMOND`].
fn distance_to_convex_polygon(x: f32, y: f32, vertices: &[(f32, f32)]) -> f32 {
    let successors = vertices.iter().cycle().skip(1);
    vertices
        .iter()
        .zip(successors)
        .map(|(start, end)| {
            let (edge_x, edge_y) = (end.0 - start.0, end.1 - start.1);
            let length = edge_x.hypot(edge_y);
            ((x - start.0) * edge_y - (y - start.1) * edge_x) / length
        })
        .fold(f32::NEG_INFINITY, f32::max)
}

fn distance_to_shape(status: GlobalStatus, x: f32, y: f32) -> f32 {
    match status {
        GlobalStatus::Idle => distance_to_ring(x, y),
        GlobalStatus::Healthy => distance_to_circle(x, y, DISC_RADIUS),
        GlobalStatus::Degraded => distance_to_convex_polygon(x, y, &TRIANGLE),
        GlobalStatus::Error => distance_to_convex_polygon(x, y, &DIAMOND),
    }
}

fn fill_color(status: GlobalStatus) -> [u8; 3] {
    match status {
        GlobalStatus::Idle => IDLE_COLOR,
        GlobalStatus::Healthy => HEALTHY_COLOR,
        GlobalStatus::Degraded => DEGRADED_COLOR,
        GlobalStatus::Error => ERROR_COLOR,
    }
}

fn classify(status: GlobalStatus, x: f32, y: f32) -> Sample {
    let distance = distance_to_shape(status, x, y);
    if distance > 0.0 {
        Sample::Outside
    } else if distance > -OUTLINE_WIDTH {
        Sample::Outline
    } else {
        Sample::Fill
    }
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
            let color = match classify(status, x, y) {
                Sample::Outside => continue,
                Sample::Outline => OUTLINE_COLOR,
                Sample::Fill => fill_color(status),
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

/// The icon of a status: a 32×32 anti-aliased picture.
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
    use std::collections::HashMap;

    use super::*;

    const ALL_STATUSES: [GlobalStatus; 4] = [
        GlobalStatus::Idle,
        GlobalStatus::Healthy,
        GlobalStatus::Degraded,
        GlobalStatus::Error,
    ];
    const OPAQUE_THRESHOLD: u8 = 128;
    const MEANINGFUL_SHAPE_DIFFERENCE_IN_PIXELS: usize = 60;
    const HUE_TOLERANCE_DEGREES: f32 = 12.0;

    fn silhouette(status: GlobalStatus) -> Vec<bool> {
        render_icon(status)
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[3] >= OPAQUE_THRESHOLD)
            .collect()
    }

    fn differing_pixels(first: GlobalStatus, second: GlobalStatus) -> usize {
        silhouette(first)
            .into_iter()
            .zip(silhouette(second))
            .filter(|(left, right)| left != right)
            .count()
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

    fn dominant_fill_color(status: GlobalStatus) -> [u8; 3] {
        let mut occurrences: HashMap<[u8; 3], usize> = HashMap::new();
        for pixel in render_icon(status).rgba.as_chunks::<4>().0 {
            let color = [pixel[0], pixel[1], pixel[2]];
            if pixel[3] == u8::MAX && color != OUTLINE_COLOR {
                *occurrences.entry(color).or_default() += 1;
            }
        }
        occurrences
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(color, _)| color)
            .unwrap()
    }

    fn assert_hue_near(status: GlobalStatus, expected_degrees: f32) {
        let hue = hue_degrees(dominant_fill_color(status));
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
    fn should_draw_different_shapes_for_every_pair_of_statuses() {
        for (index, first) in ALL_STATUSES.into_iter().enumerate() {
            for second in ALL_STATUSES.into_iter().skip(index + 1) {
                assert!(
                    differing_pixels(first, second) >= MEANINGFUL_SHAPE_DIFFERENCE_IN_PIXELS,
                    "{first:?} and {second:?} look alike"
                );
            }
        }
    }

    #[test]
    fn should_leave_the_hole_of_the_idle_ring_transparent() {
        let icon = render_icon(GlobalStatus::Idle);
        let center_alpha = icon.rgba[((16 * ICON_SIZE + 16) * 4 + 3) as usize];

        assert_eq!(center_alpha, 0);
    }

    #[test]
    fn should_leave_the_corners_transparent() {
        for status in ALL_STATUSES {
            assert_eq!(render_icon(status).rgba[3], 0, "{status:?}");
        }
    }

    #[test]
    fn should_use_a_grey_fill_when_idle() {
        let [red, green, blue] = dominant_fill_color(GlobalStatus::Idle);

        assert!(red == green && green == blue);
    }

    #[test]
    fn should_use_a_green_fill_when_healthy() {
        assert_hue_near(GlobalStatus::Healthy, 130.0);
    }

    #[test]
    fn should_use_an_amber_fill_when_degraded() {
        assert_hue_near(GlobalStatus::Degraded, 38.0);
    }

    #[test]
    fn should_use_a_red_fill_when_in_error() {
        assert_hue_near(GlobalStatus::Error, 353.0);
    }
}
