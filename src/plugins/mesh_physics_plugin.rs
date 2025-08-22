use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

/// A plugin that adds physics components to the parents of mesh entities whose names have the prefix "collider_".
/// They will be added with body type of [`RigidBody::KinematicPositionBased`]
pub struct MeshPhysicsPlugin;

impl Plugin for MeshPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, insert_physics);
    }
}

/// This system adds physics components to the parents of meshes
/// imported from glTF whose names start with "collider_".
/// It runs only once, one frame after the scene is loaded.
/// Note: We intentionally delay execution by one frame after loading
/// because [`ChildOf`] components are not yet available immediately after the scene loads.
pub fn insert_physics(
    mut commands: Commands,
    mut scene_events: EventReader<AssetEvent<Scene>>,
    meshes: Res<Assets<Mesh>>,
    mesh_query: Query<(&ChildOf, &Name, &Mesh3d)>,
    mut should_run: Local<bool>,
) {
    for event in scene_events.read() {
        let AssetEvent::LoadedWithDependencies { id: _ } = event else {
            *should_run = true;
            return;
        };
    }

    if !*should_run {
        return;
    }

    let mut sum = 0;
    // Insert physics to parents
    for (child_of, _, mesh3d) in mesh_query
        .iter()
        .filter(|(_, name, _)| name.starts_with("collider_"))
    {
        let mesh = meshes.get(mesh3d.id()).unwrap();
        let collider = Collider::from_bevy_mesh(mesh, &ComputedColliderShape::default()).unwrap();

        // Insert the physics components to the entity's parent, not the entity itself
        commands.entity(child_of.parent()).insert((
            RigidBody::KinematicPositionBased, // Some bodies may move with animation, so don't use Fixed.
            collider,
            Restitution::new(0.8),
        ));

        sum += 1;
    }
    info!("Inserted {sum} physics from scene");

    *should_run = false;
}
