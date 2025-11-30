alias Position = vec3<f32>;
alias Direction = vec3<f32>;
alias Color = vec3<f32>;

struct Object {
    distance: f32,
    color: Color,
}

fn object(distance: f32, color: Color) -> Object {
    var object: Object;
    object.distance = distance;
    object.color = color;
    return object;
}

fn sphere(position: Position, radius: f32) -> f32 {
    return length(position) - radius;
}

fn max_comp(a: vec3<f32>) -> f32 {
    return max(max(a.x, a.y), a.z);
}

fn box(position: Position, size: vec3<f32>) -> f32 {
    let q = abs(position) - size;
    return length(max(q, vec3(0))) + min(max_comp(q), 0);
}

fn object_union(a: Object, b: Object) -> Object {
    let distance = min(a.distance, b.distance);
    let factor = step(b.distance, distance);
    let color = mix(a.color, b.color, factor);
    return object(distance, color);
}

fn scene(position: vec3<f32>) -> Object {
    var p = position;
    return object_union(
        object(sphere(p, 0.5), vec3(0, 1, 0)),
        object(box(p - vec3(0.5, 0.5, -0.5), vec3(0.1)), vec3(1, 0, 0)),
    );
}

const MAX_TOTAL_DISTANCE = f32(1.0e3);
const MIN_DISTANCE = f32(5.0e-4);
const MAX_ITERATIONS = u32(3.0e2);

const BACKGROUND_COLOR = vec3<f32>(0);
const SUN_DIRECTION = vec3(-1, -0.5, 1);
const SHADOW_FACTOR = 0.7;
const SELF_SHADOW_SHARPNESS = 10;
const OBJECT_SHADOW_SHARPNESS = 32;

const INFINITY = pow(10, 20);

struct Parameters {
    camera_matrix: mat4x4<f32>,
    aspect_scale: vec2<f32>,
    time: f32,
}

@group(0) @binding(0) var<uniform> parameters: Parameters;

struct MarchResult {
    position: Position,
    distance: f32,
    color: Color,
    closeness: f32,
}

fn march(start_position: Position, direction: Direction) -> MarchResult {
    var result: MarchResult;
    result.position = start_position;
    result.distance = -INFINITY;
    result.color = BACKGROUND_COLOR;
    var total_distance: f32 = 0;
    var closeness = INFINITY;
    for (var iteration = 0u; iteration < MAX_ITERATIONS && total_distance < MAX_TOTAL_DISTANCE; iteration++) {
        let position = start_position + total_distance * direction;
        let object = scene((vec4(position, 1) * parameters.camera_matrix).xyz);
        closeness = min(closeness, object.distance / total_distance);
        if (object.distance <= MIN_DISTANCE) {
            result.color = object.color;
            result.distance = total_distance;
            result.position = position;
            break;
        }
        total_distance += object.distance;
    }
    result.closeness = closeness;
    return result;
}

fn calculate_normal(position: Position) -> Direction {
    let k = vec2<f32>(1, -1);
    return normalize(k.xyy * scene(position + k.xyy * MIN_DISTANCE).distance +
                     k.yyx * scene(position + k.yyx * MIN_DISTANCE).distance +
                     k.yxy * scene(position + k.yxy * MIN_DISTANCE).distance +
                     k.xxx * scene(position + k.xxx * MIN_DISTANCE).distance);
}

@fragment
fn fragment_main(@location(0) screen_position: vec2<f32>) -> @location(0) vec4<f32> {
    let direction = normalize(vec3(screen_position * parameters.aspect_scale, 5));
    let object_result = march(vec3(0), direction);
    var color = object_result.color;
    if (object_result.distance >= 0) {
        let position = object_result.position;
        let normal = calculate_normal(position);
        let to_sun = normalize(-SUN_DIRECTION);
        let self_shadow = SELF_SHADOW_SHARPNESS * dot(normal, to_sun);
        let sun_result = march(position + normal * 2 * MIN_DISTANCE, to_sun);
        let object_shadow = OBJECT_SHADOW_SHARPNESS * sun_result.closeness;
        let shadow = min(self_shadow, object_shadow);
        color *= mix(SHADOW_FACTOR, 1, clamp(shadow, 0, 1));
    }
    return vec4(color, 1);
}
