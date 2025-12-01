alias Scalar = f32;
alias Distance = Scalar;
alias Vector = vec3<Scalar>;
alias Homogeneous = vec4<Scalar>;
alias Position = Vector;
alias Direction = Vector;
alias Color = Vector;

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

fn max_comp(a: Vector) -> Scalar {
    return max(max(a.x, a.y), a.z);
}

fn box(position: Position, size: Vector) -> Distance {
    let q = abs(position) - size;
    return length(max(q, vec3(0))) + min(max_comp(q), 0);
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
    let num_iterations = 6u;
    let base_scale_factor = 0.1;
    let scale_factor = base_scale_factor / Scalar(1 << num_iterations);
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
    for (var i = i32(num_iterations) - 1; i >= 0; i--) {
        let distance = Scalar(1 << u32(i));
        p = mirror(p, top + distance * a_top, a_normal);
        p = mirror(p, top + distance * b_top, b_normal);
        p = mirror(p, top + distance * c_top, c_normal);
    }
    return object(tetrahedron(p, top, a, b, c), vec3(0.8, 0.8, 0.7));
}

fn test_scene(position: Position) -> Object {
    return object_union(
        object(sphere(position, 0.5), vec3(0, 1, 0)),
        object(box(position - vec3(0.5, 0.5, -0.5), vec3(0.1)), vec3(1, 0, 0)));
}

fn scene(position: Position) -> Object {
    return sierpinski_tetrahedron(position);
}

const MAX_TOTAL_DISTANCE = Distance(1.0e3);
const MIN_DISTANCE = Distance(5.0e-5);
const MAX_ITERATIONS = u32(5.0e2);

const BACKGROUND_COLOR = Color(0);
const SUN_DIRECTION = Direction(-1, -0.5, 1);
const SHADOW_FACTOR = 0.7;
const SELF_SHADOW_SHARPNESS = 10;
const OBJECT_SHADOW_SHARPNESS = 32;

const INFINITY = pow(10, 20);

struct Parameters {
    camera_matrix: mat4x4<Scalar>,
    aspect_scale: vec2<Scalar>,
    time: Scalar,
}

@group(0) @binding(0) var<uniform> parameters: Parameters;

struct MarchResult {
    position: Position,
    distance: Distance,
    color: Color,
    closeness: Scalar,
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
        let object = scene(position);
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
    let direction = transform_direction(normalize(vec3(screen_position * parameters.aspect_scale, 5)));
    let position = transform_position(vec3(0));
    let object_result = march(position, direction);
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
