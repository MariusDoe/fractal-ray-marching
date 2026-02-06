alias Scalar = f32;
alias Distance = Scalar;
alias Vector = vec3<Scalar>;
alias Homogeneous = vec4<Scalar>;
alias Position = Vector;
alias Direction = Vector;
alias Color = Vector;

const MILKY_WHITE = Color(0.8, 0.8, 0.7);

struct Object {
    distance: Distance,
    color: Color,
}

fn object(distance: Distance, color: Color) -> Object {
    var object: Object;
    object.distance = distance;
    object.color = color;
    return object;
}

fn sphere(position: Position, radius: Distance) -> Distance {
    return length(position) - radius;
}

fn max_comp2(a: vec2<Scalar>) -> Scalar {
    return max(a.x, a.y);
}

fn min_comp2(a: vec2<Scalar>) -> Scalar {
    return min(a.x, a.y);
}

fn max_comp3(a: Vector) -> Scalar {
    return max(max_comp2(a.xy), a.z);
}

fn min_comp3(a: Vector) -> Scalar {
    return min(min_comp2(a.xy), a.z);
}

fn box(position: Position, size: Vector) -> Distance {
    let q = abs(position) - size;
    return length(max(q, vec3(0))) + min(max_comp3(q), 0);
}

fn half_space(position: Position, anchor: Position, normal: Direction) -> Distance {
    return dot(position - anchor, normal);
}

fn plane_normal(a: Position, b: Position, c: Position) -> Direction {
    return normalize(cross(c - a, b - a));
}

fn tetrahedron(position: Position, a: Position, b: Position, c: Position, d: Position) -> Distance {
    return max(max(max(
        half_space(position, a, plane_normal(a, b, c)),
        half_space(position, a, plane_normal(a, c, d))),
        half_space(position, a, plane_normal(a, d, b))),
        half_space(position, b, plane_normal(b, d, c)));
}

fn object_union(a: Object, b: Object) -> Object {
    let distance = min(a.distance, b.distance);
    let factor = step(b.distance, distance);
    let color = mix(a.color, b.color, factor);
    return object(distance, color);
}

fn mirror(position: Position, anchor: Position, normal: Direction) -> Position {
    let distance = dot(position - anchor, normal);
    return position + (abs(distance) - distance) * normal;
}

fn sierpinski_tetrahedron(position: Position) -> Object {
    let base_scale_factor = 0.5;
    let scale_factor = base_scale_factor / Scalar(1 << parameters.num_iterations);
    let height = 4 / sqrt(6);
    let top = vec3(0, height * base_scale_factor, 0);
    let one_over_sqrt_3 = 1 / sqrt(3);
    let a = top + scale_factor * vec3(-1, -height, -one_over_sqrt_3);
    let b = top + scale_factor * vec3(1, -height, -one_over_sqrt_3);
    let c = top + scale_factor * vec3(0, -height, 2 * one_over_sqrt_3);
    let a_top = a - top;
    let b_top = b - top;
    let c_top = c - top;
    let a_normal = normalize(top - a);
    let b_normal = normalize(top - b);
    let c_normal = normalize(top - c);
    var p = position;
    p.y += height * base_scale_factor * 0.5;
    for (var i = i32(parameters.num_iterations) - 1; i >= 0; i--) {
        let distance = Scalar(1 << u32(i));
        p = mirror(p, top + distance * a_top, a_normal);
        p = mirror(p, top + distance * b_top, b_normal);
        p = mirror(p, top + distance * c_top, c_normal);
    }
    return object(tetrahedron(p, top, a, b, c), MILKY_WHITE);
}

fn repeat(position: Position) -> Position {
    return fract(position + 0.5) - 0.5;
}

fn cross_inside(position: Position, size: Distance) -> Distance {
    let p = abs(position);
    let x = max_comp2(p.yz);
    let y = max_comp2(p.zx);
    let z = max_comp2(p.xy);
    return min_comp3(vec3(x, y, z)) - size;
}

fn menger_sponge(position: Position) -> Object {
    let size = 0.5;
    var distance = box(position, vec3(size));
    var scale = 0.5 / size;
    for (var i = 0u; i < parameters.num_iterations; i++) {
        distance = max(distance, -cross_inside(repeat(position * scale), 1.0 / 6.0) / scale);
        scale *= 3.0;
    }
    return object(distance, MILKY_WHITE);
}

fn test_scene(position: Position) -> Object {
    return object_union(
        object(sphere(position, 0.5), vec3(0, 1, 0)),
        object(box(position - vec3(0.5, 0.5, -0.5), vec3(0.1)), vec3(1, 0, 0)));
}

fn scene(position: Position) -> Object {
    switch (parameters.scene_index) {
        case 0, default: {
            return menger_sponge(position);
        }
        case 1: {
            return sierpinski_tetrahedron(position);
        }
    }
}

const MAX_TOTAL_DISTANCE = Distance(1.0e3);
const MIN_DISTANCE = Distance(5.0e-7);
const MAX_ITERATIONS = u32(5.0e3);

const BACKGROUND_COLOR = Color(0);
const SUN_DIRECTION = Direction(-1, -0.5, 1);
const SUN_COLOR = Color(1);
const SHADOW_FACTOR = 0.7;
const SELF_SHADOW_SHARPNESS = 10;
const OBJECT_SHADOW_SHARPNESS = 32;

const INFINITY = pow(10, 20);

struct Parameters {
    camera_matrix: mat4x4<Scalar>,
    aspect_scale: vec2<Scalar>,
    time: Scalar,
    num_iterations: u32,
    scene_index: u32,
}

@group(0) @binding(0) var<uniform> parameters: Parameters;

struct MarchResult {
    position: Position,
    distance: Distance,
    color: Color,
    closest_distance: Distance,
    steps: u32,
    hit: bool,
}

fn march(start_position: Position, direction: Direction) -> MarchResult {
    var position = start_position;
    var closest_distance = INFINITY;
    var color = Color(0);
    var total_distance: f32 = 0;
    var iteration = 0u;
    var hit = false;
    for (; iteration < MAX_ITERATIONS && total_distance < MAX_TOTAL_DISTANCE; iteration++) {
        position = start_position + total_distance * direction;
        let object = scene(position);
        closest_distance = min(closest_distance, object.distance);
        if (closest_distance == object.distance) {
            color = object.color;
        }
        if (object.distance <= MIN_DISTANCE) {
            hit = true;
            break;
        }
        total_distance += object.distance;
    }
    var result: MarchResult;
    result.position = position;
    result.distance = total_distance;
    result.closest_distance = closest_distance;
    result.color = color;
    result.steps = iteration;
    result.hit = hit;
    return result;
}

fn calculate_normal(position: Position) -> Direction {
    // adapted from here (Tetrahedron technique): https://iquilezles.org/articles/normalsSDF/
    let k = vec2<Scalar>(1, -1);
    return normalize(k.xyy * scene(position + k.xyy * MIN_DISTANCE).distance +
                     k.yyx * scene(position + k.yyx * MIN_DISTANCE).distance +
                     k.yxy * scene(position + k.yxy * MIN_DISTANCE).distance +
                     k.xxx * scene(position + k.xxx * MIN_DISTANCE).distance);
}

fn transform_homogeneous(a: Homogeneous) -> Vector {
    return (a * parameters.camera_matrix).xyz;
}

fn transform_position(position: Position) -> Position {
    return transform_homogeneous(vec4(position, 1));
}

fn transform_direction(direction: Direction) -> Direction {
    return transform_homogeneous(vec4(direction, 0));
}

@fragment
fn fragment_main(@location(0) screen_position: vec2<Scalar>) -> @location(0) vec4<Scalar> {
    let camera_direction = transform_direction(normalize(vec3(screen_position * parameters.aspect_scale, 5)));
    let camera_position = transform_position(vec3(0));
    let object_result = march(camera_position, camera_direction);
    var color = BACKGROUND_COLOR;
    if (object_result.hit) {
        color = object_result.color;
        let object_position = object_result.position;
        let object_normal = calculate_normal(object_position);
        let to_sun = normalize(-SUN_DIRECTION);
        let self_shadow = SELF_SHADOW_SHARPNESS * dot(object_normal, to_sun);
        let to_camera = -camera_direction;
        let halfway = normalize(to_camera + to_sun);
        let specular = pow(max(dot(halfway, object_normal), 0), 16);
        let sun_result = march(object_position + object_normal * 2 * MIN_DISTANCE, to_sun);
        let object_shadow = OBJECT_SHADOW_SHARPNESS * sun_result.closest_distance / sun_result.distance;
        let shadow = min(self_shadow, object_shadow);
        color *= mix(0.2, 1.0, pow(1.0 - f32(object_result.steps) / f32(MAX_ITERATIONS), 60));
        color *= mix(SHADOW_FACTOR, 1, clamp(shadow, 0, 1));
        color += shadow * specular * SUN_COLOR;
    } else {
        color += object_result.color * 5 * max(0.02 - object_result.closest_distance, 0);
    }
    return vec4(color, 1);
}
