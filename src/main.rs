// Importation des bibliothèques.
use bevy::{ // Bibliothèque de moteur de jeu Bevy
            prelude::*,
            render::{
                extract_resource::{ExtractResource, ExtractResourcePlugin},
                render_asset::{RenderAssetUsages, RenderAssets},
                render_graph::{self, RenderGraph, RenderLabel},
                render_resource::{binding_types::texture_storage_2d, *},
                renderer::{RenderContext, RenderDevice},
                texture::GpuImage,
                Render, RenderApp, RenderSet,
            },
            app::AppExit,
            window::WindowMode,
            diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin}, // Plugins pour les logs des performances
};
use std::borrow::Cow;

const SHADER_ASSET_PATH: &str = "shaders/compute.wgsl"; // Chemin du shader
const DISPLAY_FACTOR: u32 = 4; // Taille d'une cellule en pixel.
const SIZE: (u32, u32) = (1920 / DISPLAY_FACTOR, 1080 / DISPLAY_FACTOR);
const WORKGROUP_SIZE: u32 = 8;
const EPISODE_STEP_DURATION: f32 = 0.07;

fn main() {
    // Instanciation de l'application avec Bevy.
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(SimulationTimer(Timer::from_seconds(EPISODE_STEP_DURATION, TimerMode::Repeating)))
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Lenia".into(),
                        resolution: (
                            (SIZE.0 * DISPLAY_FACTOR) as f32,
                            (SIZE.1 * DISPLAY_FACTOR) as f32,
                        ).into(),
                        present_mode: bevy::window::PresentMode::AutoNoVsync,
                        mode: WindowMode::BorderlessFullscreen,
                        enabled_buttons: bevy::window::EnabledButtons {
                            maximize: false,
                            ..Default::default()
                        },
                        //visible: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()), // Polissage des défauts de pixel.
            GameOfLifeComputePlugin,
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_simulation_timer,
                switch_textures.after(update_simulation_timer),
                exit_on_esc_system
            ),
        )
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Instanciation d'une texture.
    let mut image = Image::new_fill(
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
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;

    // Une texture pour chaque état (précédent et suivant)
    let image0 = images.add(image.clone()); // Clone l'instanciation de la texture 
    let image1 = images.add(image);

    // Instancie un Sprite dans le monde 2d.
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..default()
        },
        texture: image0.clone(),
        transform: Transform::from_scale(Vec3::splat(DISPLAY_FACTOR as f32)),
        ..default()
    });

    // Instancie la caméra
    commands.spawn(Camera2dBundle::default());

    // Insère la ressource qui contient les textures (deux états)
    commands.insert_resource(GameOfLifeImages {
        texture_a: image0,
        texture_b: image1,
    });
}

// Fonction pour interchanger les textures à chaque frame
fn switch_textures(images: Res<GameOfLifeImages>, mut displayed: Query<&mut Handle<Image>>) {
    let mut displayed = displayed.single_mut();
    if *displayed == images.texture_a {
        *displayed = images.texture_b.clone_weak();
    } else {
        *displayed = images.texture_a.clone_weak();
    }
}

// Création d'un plugin pour gérer la computation du shader (bind groups, pipeline et nodes)
struct GameOfLifeComputePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GameOfLifeLabel;

impl Plugin for GameOfLifeComputePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<GameOfLifeImages>::default());
        app.add_plugins(ExtractResourcePlugin::<SimulationTimer>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            prepare_bind_group.in_set(RenderSet::PrepareBindGroups),
        );

        let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        render_graph.add_node(GameOfLifeLabel, GameOfLifeNode::default());
        render_graph.add_node_edge(GameOfLifeLabel, bevy::render::graph::CameraDriverLabel);
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<GameOfLifePipeline>();
    }
}

// Structure pour les textures
#[derive(Resource, Clone, ExtractResource)]
struct GameOfLifeImages {
    texture_a: Handle<Image>,
    texture_b: Handle<Image>,
}

// Texture contenant les bind group
#[derive(Resource)]
struct GameOfLifeImageBindGroups([BindGroup; 2]);

// Préparation des bind group
fn prepare_bind_group(
    mut commands: Commands,
    pipeline: Res<GameOfLifePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    game_of_life_images: Res<GameOfLifeImages>,
    render_device: Res<RenderDevice>,
) {
    let view_a = gpu_images.get(&game_of_life_images.texture_a).unwrap();
    let view_b = gpu_images.get(&game_of_life_images.texture_b).unwrap();
    let bind_group_0 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_a.texture_view, &view_b.texture_view)),
    );
    let bind_group_1 = render_device.create_bind_group(
        None,
        &pipeline.texture_bind_group_layout,
        &BindGroupEntries::sequential((&view_b.texture_view, &view_a.texture_view)),
    );
    commands.insert_resource(GameOfLifeImageBindGroups([bind_group_0, bind_group_1]));
}

// Structure du Pipeline qui sera créé
#[derive(Resource)]
struct GameOfLifePipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

// Création du Pipeline du shader
impl FromWorld for GameOfLifePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let texture_bind_group_layout = render_device.create_bind_group_layout(
            "GameOfLifeImages",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::ReadOnly),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                ),
            ),
        );
        let shader = world.load_asset(SHADER_ASSET_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![texture_bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        GameOfLifePipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

// Etat dans lequel le jeu passera pour vérifier la présence de ressources buffer, initialisation des entités, etc..
#[derive(Debug)]
enum GameOfLifeState {
    Loading,
    Init,
    Update(usize),
}

// Création d'un node d'état
struct GameOfLifeNode {
    state: GameOfLifeState,
}

impl Default for GameOfLifeNode {
    fn default() -> Self {
        Self {
            state: GameOfLifeState::Loading,
        }
    }
}

// Met en place le render graph de l'application en explicitant les transitions des états avec les nodes
impl render_graph::Node for GameOfLifeNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GameOfLifePipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let timer = world.resource::<SimulationTimer>();

        //println!("Current state: {:?}", self.state);

        match self.state {
            GameOfLifeState::Loading => {
                match pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline) {
                    CachedPipelineState::Ok(_) => {
                        self.state = GameOfLifeState::Init;
                    }
                    CachedPipelineState::Err(err) => {
                        panic!("Initializing assets/{SHADER_ASSET_PATH}:\n{err}")
                    }
                    _ => {}
                }
            }
            GameOfLifeState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = GameOfLifeState::Update(1);
                }
            }
            GameOfLifeState::Update(_) => {
                if timer.0.finished() {
                    // Alternez entre les états pour échanger les textures
                    self.state = match self.state {
                        GameOfLifeState::Update(0) => GameOfLifeState::Update(1),
                        GameOfLifeState::Update(1) => GameOfLifeState::Update(0),
                        _ => unreachable!(),
                    };
                }
            }
        }
    }

    // Lance le compute
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let bind_groups = &world.resource::<GameOfLifeImageBindGroups>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GameOfLifePipeline>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        match self.state {
            GameOfLifeState::Loading => {}
            GameOfLifeState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[0], &[]);
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            GameOfLifeState::Update(index) => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_bind_group(0, &bind_groups[index], &[]);
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}

#[derive(Resource, Clone, ExtractResource, Default)]
struct SimulationTimer(Timer);

fn update_simulation_timer(time: Res<Time>, mut timer: ResMut<SimulationTimer>) {
    timer.0.tick(time.delta());
}

fn exit_on_esc_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut exit: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}