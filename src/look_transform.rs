use bevy::{
    app::prelude::*,
    ecs::{bundle::Bundle, prelude::*},
    math::prelude::*,
    transform::components::Transform,
};

pub struct LookTransformPlugin;

impl Plugin for LookTransformPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(look_transform_system);
    }
}

#[derive(Bundle)]
pub struct LookTransformBundle {
    pub transform: LookTransform,
    pub smoother: Smoother,
}

/// An eye and the target it's looking at. As a component, this can be modified in place of bevy's `Transform`, and the two will
/// stay in sync.
#[derive(Clone, Component, Copy, Debug)]
pub struct LookTransform {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub(crate) enabled: bool,
}

impl From<LookTransform> for Transform {
    fn from(t: LookTransform) -> Self {
        eye_look_at_target_transform(t.eye, t.target, t.up)
    }
}

impl LookTransform {
    pub fn new(eye: Vec3, target: Vec3) -> Self {
        Self {
            eye,
            target,
            up: Vec3::Y,
            enabled: true,
        }
    }

    pub fn radius(&self) -> f32 {
        (self.target - self.eye).length()
    }

    pub fn look_direction(&self) -> Option<Vec3> {
        (self.target - self.eye).try_normalize()
    }
}

fn eye_look_at_target_transform(eye: Vec3, target: Vec3, up: Vec3) -> Transform {
    // If eye and target are very close, we avoid imprecision issues by keeping the look vector a unit vector.
    let look_vector = (target - eye).normalize();
    let look_at = eye + look_vector;

    Transform::from_translation(eye).looking_at(look_at, up)
}

/// Preforms exponential smoothing on a `LookTransform`. Set the `lag_weight` between `0.0` and `1.0`, where higher is smoother.
#[derive(Component)]
pub struct Smoother {
    lag_weight: f32,
    lerp_tfm: Option<LookTransform>,
}

impl Smoother {
    pub fn new(lag_weight: f32) -> Self {
        Self {
            lag_weight,
            lerp_tfm: None,
        }
    }

    pub fn set_lag_weight(&mut self, lag_weight: f32) {
        self.lag_weight = lag_weight;
    }

    pub fn smooth_transform(&mut self, new_tfm: &LookTransform) -> LookTransform {
        debug_assert!(0.0 <= self.lag_weight);
        debug_assert!(self.lag_weight < 1.0);

        let old_lerp_tfm = self.lerp_tfm.unwrap_or_else(|| *new_tfm);
        let lerp_tfm = if new_tfm.enabled && old_lerp_tfm.enabled {
            let lead_weight = 1.0 - self.lag_weight;

            LookTransform {
                eye: old_lerp_tfm.eye * self.lag_weight + new_tfm.eye * lead_weight,
                target: old_lerp_tfm.target * self.lag_weight + new_tfm.target * lead_weight,
                ..*new_tfm
            }
        } else {
            // Don't apply any interpolation if we were disabled now or past frame.
            // This is to allow external systems to disable, modify the position of the camera
            // manually then re-enable it without animating this transition.
            *new_tfm
        };

        self.lerp_tfm = Some(lerp_tfm);

        lerp_tfm
    }
}

fn look_transform_system(
    mut cameras: Query<(&LookTransform, &mut Transform, Option<&mut Smoother>)>,
) {
    for (look_transform, mut scene_transform, smoother) in cameras.iter_mut() {
        let effective_look_transform = if let Some(mut smoother) = smoother {
            smoother.smooth_transform(look_transform)
        } else {
            *look_transform
        };

        if look_transform.enabled {
            *scene_transform = effective_look_transform.into();
        }
    }
}
