use bevy::{
    math::vec2,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssetUsages,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph, RenderLabel},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        texture::GpuImage,
        Render, RenderApp, RenderSet,
    },
};

const SIZE: (u32, u32) = (1200, 800);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.5, 0.5, 0.5)))
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            GameOfLifeComputePlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, update_view)
        .add_systems(Update, flip_textures)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut read_image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    );
    read_image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let mut write_image = read_image.clone();
    let idx = ((SIZE.0 / 2 - 1) * 4) as usize;
    write_image.data[idx] = 255;
    write_image.data[idx + 1] = 255;
    write_image.data[idx + 2] = 255;
    write_image.data[idx + 3] = 255;
    let read_image = images.add(read_image);
    let write_image = images.add(write_image);

    commands.spawn(Sprite {
        custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
        image: read_image.clone(),
        ..default()
    });
    commands.spawn(Camera2d::default());

    commands.insert_resource(GameOfLifeImage {
        read_texture: read_image,
        write_texture: write_image,
    });
}

struct GameOfLifeComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GameOfLifeLabel;

impl Plugin for GameOfLifeComputePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<GameOfLifeImage>::default());
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(GameOfLifeLabel, GameOfLifeNode);
        render_graph.add_node_edge(GameOfLifeLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GameOfLifePipeline>();
    }
}

#[derive(Resource, Clone, ExtractResource, AsBindGroup)]
struct GameOfLifeImage {
    #[storage_texture(0, image_format = Rgba8Unorm, access = ReadOnly)]
    read_texture: Handle<Image>,
    #[storage_texture(1, image_format = Rgba8Unorm, access = WriteOnly)]
    write_texture: Handle<Image>,
}

#[derive(Resource)]
struct GameOfLifeImageBindGroup(BindGroup);

fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<GameOfLifePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    game_of_life_image: Res<GameOfLifeImage>,
    render_device: Res<RenderDevice>,
) {
    let read_view = gpu_images.get(&game_of_life_image.read_texture).unwrap();
    let write_view = gpu_images.get(&game_of_life_image.write_texture).unwrap();
    let bind_group = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&read_view.texture_view, &write_view.texture_view)),
    );
    commands.insert_resource(GameOfLifeImageBindGroup(bind_group));
}

#[derive(Resource)]
struct GameOfLifePipeline {
    texture_bind_group_layout: BindGroupLayout,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for GameOfLifePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = GameOfLifeImage::bind_group_layout(render_device);
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/game_of_life.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: "update".into(),
            zero_initialize_workgroup_memory: true,
        });

        GameOfLifePipeline {
            texture_bind_group_layout,
            update_pipeline,
        }
    }
}

struct GameOfLifeNode;

impl render_graph::Node for GameOfLifeNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<GameOfLifeImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GameOfLifePipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        if let Some(update_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.update_pipeline)
        {
            pass.set_pipeline(update_pipeline);
            pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
        }

        Ok(())
    }
}

fn update_view(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Sprite>>,
) {
    let scale_speed = 1.0;
    let mut scale_dir = 0.0;
    if input.pressed(KeyCode::ShiftLeft) {
        scale_dir -= 1.0;
    }
    if input.pressed(KeyCode::ControlLeft) {
        scale_dir += 1.0;
    }
    let scale_amount = scale_dir * scale_speed * time.delta_secs();

    let move_speed = 1000.0;
    let mut move_dir = vec2(0.0, 0.0);
    if input.pressed(KeyCode::KeyW) {
        move_dir.y -= 1.0;
    }
    if input.pressed(KeyCode::KeyA) {
        move_dir.x += 1.0;
    }
    if input.pressed(KeyCode::KeyS) {
        move_dir.y += 1.0;
    }
    if input.pressed(KeyCode::KeyD) {
        move_dir.x -= 1.0;
    }
    let move_amount = move_dir * move_speed * time.delta_secs();

    for mut tf in query.iter_mut() {
        tf.translation.x += move_amount.x;
        tf.translation.y += move_amount.y;
        tf.scale.x += scale_amount;
        tf.scale.y = -tf.scale.x;
    }
}

fn flip_textures(mut textures: ResMut<GameOfLifeImage>, mut query: Query<&mut Sprite>) {
    let temp = textures.read_texture.clone();
    textures.read_texture = textures.write_texture.clone();
    textures.write_texture = temp;

    for mut sprite in query.iter_mut() {
        sprite.image = textures.read_texture.clone();
    }
}
