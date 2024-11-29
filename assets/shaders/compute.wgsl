@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

const R: i32 = 2; // Rayon du noyau étendu
const b1: i32 = 3; // Limite inférieure de la plage de naissance
const b2: i32 = 4; // Limite supérieure de la plage de naissance
const s1: i32 = 2; // Limite inférieure de la plage de stabilité
const s2: i32 = 5; // Limite supérieure de la plage de stabilité

const MAX_STATES: i32 = 12;

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

// Initialisation de la grille avec des cellules vivantes ou mortes aléatoires
@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let randomNumber = randomFloat(invocation_id.y << 16u | invocation_id.x);
    let state = i32(randomNumber * f32(MAX_STATES + 1));
    let color = vec4<f32>(f32(state) / f32(MAX_STATES), 0.0, 0.0, 1.0);
    textureStore(output, location, color);
}

// Vérifie si une cellule est vivante
fn get_state(location: vec2<i32>, size: vec2<i32>) -> i32 {
    var wrapped_location = (location + size) % size; // Wrapping torique
    let value: vec4<f32> = textureLoad(input, wrapped_location);
    return i32(round(value.x * f32(MAX_STATES))); // Récupère l'état entier
}

fn apply_convolution(location: vec2<i32>, size: vec2<i32>) -> i32 {
    var sum: i32 = 0;
    for (var i = -1; i <= 1; i = i + 1) {
        for (var j = -1; j <= 1; j = j + 1) {
            if (!(i == 0 && j == 0)) { // Exclure la cellule centrale
                let neighbor = location + vec2<i32>(i, j);
                sum += get_state(neighbor, size);
            }
        }
    }
    return sum;
}


// Fonction growth pour ajuster l'état des cellules
fn growth(U: i32) -> i32 {
    let is_birth = i32(U >= 20 && U <= 24); // Naissance si U dans [20, 24]
    let is_death = i32(U <= 18 || U >= 32); // Mort si U <= 18 ou U >= 32
    return is_birth - is_death;
}

// Met à jour la grille en appliquant les règles du Jeu de la Vie
@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let size = vec2<i32>(textureDimensions(input));
    let U = apply_convolution(location, size);
    let delta = growth(U);
    let current_state = get_state(location, size);
    var new_state = clamp(current_state + delta, 0, MAX_STATES);
    var color = vec4<f32>(f32(new_state) / f32(MAX_STATES), 0.0, 0.0, 1.0);
    textureStore(output, location, color);
}

