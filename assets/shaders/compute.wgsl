@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

const MAX_STATES: i32 = 12;
const T: f32 = 10.0;

// Fonction de hachage pour générer un nombre pseudo-aléatoire
fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    return state;
}

// Conversion du hachage en un flottant entre 0 et 1
fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

// Initialisation de la grille avec des états aléatoires
@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let randomNumber = randomFloat(invocation_id.y << 16u | invocation_id.x);
    let state = randomNumber; // État entre 0 et 1
    let color = state_to_color(state);
    textureStore(output, location, color);
}

// Récupère l'état d'une cellule
fn get_state(location: vec2<i32>, size: vec2<i32>) -> f32 {
    var wrapped_location = (location + size) % size; // Wrapping torique
    let value: vec4<f32> = textureLoad(input, wrapped_location);
    return value.a; // Retourne l'état continu depuis le canal alpha
}

fn apply_convolution(location: vec2<i32>, size: vec2<i32>) -> f32 {
    var sum: f32 = 0.0;
    for (var i = -1; i <= 1; i = i + 1) {
        for (var j = -1; j <= 1; j = j + 1) {
            if (!(i == 0 && j == 0)) { // Exclure la cellule centrale
                let neighbor = location + vec2<i32>(i, j);
                sum = sum + get_state(neighbor, size);
            }
        }
    }
    return sum / 8.0; // Normalise en divisant par le nombre de voisins
}

// Fonction growth pour ajuster l'état des cellules
fn growth(U: f32) -> f32 {
    let is_birth = f32((U >= 0.20) && (U <= 0.25));
    let is_death = f32((U <= 0.19) || (U >= 0.33));
    return is_birth - is_death;
}

// Mappe l'état normalisé à une couleur (de bleu à vert à rouge)
fn state_to_color(state: f32) -> vec4<f32> {
    var color = vec3<f32>(0.0, 0.0, 0.0);
    if (state > 0.0) {
        if (state <= 0.5) {
            // De bleu à vert
            color = mix(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), state * 2.0);
        } else {
            // De vert à rouge
            color = mix(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), (state - 0.5) * 2.0);
        }
    }
    return vec4<f32>(color, state); // Stocke l'état dans le canal alpha
}

// Met à jour la grille en appliquant les règles
@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let size = vec2<i32>(textureDimensions(input));
    let U = apply_convolution(location, size);
    let delta = growth(U);
    let current_state = get_state(location, size);
    var new_state = clamp(current_state + (1.0 / T) * delta, 0.0, 1.0);
    let color = state_to_color(new_state);
    textureStore(output, location, color);
}
