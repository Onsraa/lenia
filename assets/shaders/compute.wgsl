@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

const T: f32 = 10.0; // Frequency
const R: i32 = 5; // Kernel radius

// Generate random hash
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

// Generate random number between 0 & 1
fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

// Initialize grid of cells
@compute @workgroup_size(8, 8, 1)
fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    let randomNumber = randomFloat(invocation_id.y << 16u | invocation_id.x);
    let state = randomNumber; // État entre 0 et 1
    let color = state_to_color(state);
    textureStore(output, location, color);
}


fn get_state(location: vec2<i32>, size: vec2<i32>) -> f32 {
    var wrapped_location = (location + size) % size; // Wrapping TORUS
    let value: vec4<f32> = textureLoad(input, wrapped_location);
    return value.a; // Return continious state from red channel
}

fn apply_convolution(location: vec2<i32>, size: vec2<i32>) -> f32 {
    var sum: f32 = 0.0;
    for (var i = -R; i <= R; i = i + 1) {
        for (var j = -R; j <= R; j = j + 1) {
            if (!(i == 0 && j == 0)) { // Exclude central cell
                let neighbor = location + vec2<i32>(i, j);
                sum = sum + get_state(neighbor, size);
            }
        }
    }
    let total_neighbors = f32((2 * R + 1) * (2 * R + 1) - 1);
    return sum / total_neighbors; // Normalize
}

// Growth function to adjust cell states
fn growth(U: f32) -> f32 {
    let is_growth = f32(U >= 0.12 && U <= 0.15);
    let is_decay = f32(U < 0.12 || U > 0.15);
    return is_growth - is_decay;
}

// For the beauty of life : Colors the state depending on the intensity
fn state_to_color(state: f32) -> vec4<f32> {
    var color = vec3<f32>(0.0, 0.0, 0.0);
    if (state > 0.0) {
        if (state <= 0.5) {
            // Blue (Not intense) -> Green (Ok intense)
            color = mix(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), state * 2.0);
        } else {
            // Green (Ok intense) -> Red (Very intense)
            color = mix(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), (state - 0.5) * 2.0);
        }
    }
    return vec4<f32>(color, state);
}

// Update grid with every rules applied for Lenia
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
