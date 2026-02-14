// raymarching
const MAX_TOTAL_DISTANCE = Distance(1.0e3);
const MIN_DISTANCE = Distance(5.0e-7);
const MAX_ITERATIONS = u32(5.0e3);
const FOV_DEGREES = 90;

// colors / shading
const BACKGROUND_COLOR = Color(0, 0, 0);
const SUN_DIRECTION = Direction(-1, -0.5, 1);
const SUN_COLOR = Color(1, 1, 1);
const SHADOW_FACTOR = 0.7;
const SHADOW_SHARPNESS = 32;
const SPECULAR_SHARPNESS = 16;
const AMBIENT_OCCLUSION_FACTOR = 0.2;
const AMBIENT_OCCLUSION_SHARPNESS = 100;

fn scene(position: Position) -> Object {
    switch (parameters.scene_index) {
        case 0, default: {
            return menger_sponge(position, 1.0 / 6.0, 3.0);
        }
        case 1: {
            return menger_sponge(position, 1.0 / 5.0, 3.0);
        }
        case 2: {
            return menger_sponge(position, 1.0 / 4.0, 3.0);
        }
        case 3: {
            return menger_sponge(position, 1.0 / 3.0, 3.0);
        }
        case 4: {
            return menger_sponge(position, 1.0 / animate_between(2, 8), 3.0);
        }
        case 5: {
            return menger_sponge(position, 1.0 / 6.0, 2.0);
        }
        case 6: {
            return menger_sponge(position, 1.0 / 4.0, 2.0);
        }
        case 7: {
            return menger_sponge(position, 1.0 / 8.0, 2.0);
        }
        case 8: {
            return menger_sponge(position, 1.0 / animate_between(3, 10), 2.0);
        }
        case 9: {
            return menger_sponge(position, 1.0 / 4.0, 4.0);
        }
        case 10: {
            return menger_sponge(position, 1.0 / 5.0, 5.0);
        }
        case 11: {
            return menger_sponge(position, 1.0 / 4.0, 6.0);
        }
        case 12: {
            return menger_sponge(position, 1.0 / 3.0, animate_between(3, 5));
        }
        case 13: {
            return menger_sponge(position, 1.0 / 4.0, animate_between(2, 4));
        }
        case 14: {
            return menger_sponge(position, 1.0 / 6.0, animate_between(1.2, 3));
        }
        case 15: {
            return sierpinski_tetrahedron(position);
        }
        case 16: {
            return koch3D(position, sqrt(3));
        }
        case 17: {
            return koch3D(position, animate_between(sqrt(3), 4));
        }
        case 18: {
            return mandelbulb(position, animate_between(1, 9), 100.0);
        }
    }
}

fn animate_between(a: Scalar, b: Scalar) -> Scalar {
    return a + (b - a) * (0.5 + 0.5 * sin(parameters.time * 0.2));
}

alias Scalar = f32;
alias Distance = Scalar;
alias Vector = vec3<Scalar>;
alias Homogeneous = vec4<Scalar>;
alias Position = Vector;
alias Direction = Vector;
alias Color = Vector;

const PI = 3.141592653589793238;
const TWO_PI = 2 * PI;
const INFINITY = pow(10, 20);

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

struct Parameters {
    camera_matrix: mat4x4<Scalar>,
    aspect_scale: vec2<Scalar>,
    time: Scalar,
    num_iterations: u32,
    scene_index: u32,
}

@group(0) @binding(0) var<uniform> parameters: Parameters;

fn colorize(position: Position) -> Color {
    return min(Color(1), position + 0.5);
}

fn max_component_2(a: vec2<Scalar>) -> Scalar {
    return max(a.x, a.y);
}

fn min_component_2(a: vec2<Scalar>) -> Scalar {
    return min(a.x, a.y);
}

fn max_component_3(a: Vector) -> Scalar {
    return max(max_component_2(a.xy), a.z);
}

fn min_component_3(a: Vector) -> Scalar {
    return min(min_component_2(a.xy), a.z);
}

fn box(position: Position, size: Vector) -> Distance {
    let q = abs(position) - size;
    return length(max(q, Vector(0))) + min(max_component_3(q), 0);
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

fn mirror(position: Position, anchor: Position, normal: Direction) -> Position {
    let distance = dot(position - anchor, normal);
    return position + (abs(distance) - distance) * normal;
}

fn sierpinski_tetrahedron(position: Position) -> Object {
    const BASE_SCALE_FACTOR = 0.5;
    let scale_factor = BASE_SCALE_FACTOR / Scalar(1 << parameters.num_iterations);
    const HEIGHT = 4 / sqrt(6);
    const TOP = Position(0, HEIGHT * BASE_SCALE_FACTOR, 0);
    const ONE_OVER_SQRT_3 = 1 / sqrt(3);
    let a = TOP + scale_factor * Direction(-1, -HEIGHT, -ONE_OVER_SQRT_3);
    let b = TOP + scale_factor * Direction(1, -HEIGHT, -ONE_OVER_SQRT_3);
    let c = TOP + scale_factor * Direction(0, -HEIGHT, 2 * ONE_OVER_SQRT_3);
    let a_top = a - TOP;
    let b_top = b - TOP;
    let c_top = c - TOP;
    let a_normal = normalize(TOP - a);
    let b_normal = normalize(TOP - b);
    let c_normal = normalize(TOP - c);
    var p = position;
    p.y += HEIGHT * BASE_SCALE_FACTOR * 0.5;
    for (var i = i32(parameters.num_iterations) - 1; i >= 0; i--) {
        let distance = Scalar(1 << u32(i));
        p = mirror(p, TOP + distance * a_top, a_normal);
        p = mirror(p, TOP + distance * b_top, b_normal);
        p = mirror(p, TOP + distance * c_top, c_normal);
    }
    return object(tetrahedron(p, TOP, a, b, c), colorize(1.5 * position));
}

fn repeat(position: Position) -> Position {
    return fract(position + 0.5) - 0.5;
}

fn cross_inside(position: Position, size: Distance) -> Distance {
    let p = abs(position);
    let x = max_component_2(p.yz);
    let y = max_component_2(p.zx);
    let z = max_component_2(p.xy);
    return min_component_3(Position(x, y, z)) - size;
}

fn menger_sponge(position: Position, cross_size: Scalar, scale_factor: Scalar) -> Object {
    const SIZE = 0.5;
    var distance = box(position, Vector(SIZE));
    var scale = 0.5 / SIZE;
    for (var i = 0u; i < parameters.num_iterations; i++) {
        distance = max(distance, -cross_inside(repeat(position * scale), cross_size) / scale);
        scale *= scale_factor;
    }
    return object(distance, colorize(position));
}

fn koch3D(position: Position, normal_z: Scalar) -> Object {
    const SIDE_LENGTH = Scalar(3);
    const HALF_SIDE_LENGTH = SIDE_LENGTH / 2;
    const SIDE_LENGTH_SQRT = sqrt(SIDE_LENGTH);
    const OFFSET = sqrt(SIDE_LENGTH * SIDE_LENGTH - HALF_SIDE_LENGTH * HALF_SIDE_LENGTH) - SIDE_LENGTH_SQRT;
    const TOP = Position(0, 1, 0);
    const LEFT = Position(-HALF_SIDE_LENGTH, 0, -OFFSET);
    const RIGHT = Position(HALF_SIDE_LENGTH, 0, -OFFSET);
    const BACK = Position(0, 0, SIDE_LENGTH_SQRT);
    let normal_1 = normalize(Direction(0, 1, normal_z));
    let normal_2 = normal_1 * Vector(1, -1, 1);
    var p = position;
    var scale_factor = 2.0;
    p *= scale_factor;
    for (var i = 0u; i < parameters.num_iterations; i++) {
        const FACTOR = 3.0 / 2.0;
        scale_factor *= FACTOR;
        p *= FACTOR;
        p = p.yxz;
        p = mirror(p, Position(0), normal_1);
        p = mirror(p, Position(0), normal_2);
        p.z -= OFFSET;
    }
    p.y = abs(p.y);
    return object(tetrahedron(p, TOP, LEFT, RIGHT, BACK) / scale_factor, colorize(position));
}

fn mandelbulb(position: Position, power: Scalar, bailout: Scalar) -> Object {
    // adapted from http://blog.hvidtfeldts.net/index.php/2011/09/distance-estimated-3d-fractals-v-the-mandelbulb-different-de-approximations/
    var current = position;
    var magnitude_derivative = 1.0;
    var magnitude = 0.0;
    for (var i = 0u; i <= parameters.num_iterations; i++) {
        magnitude = length(current);
        if (magnitude > bailout) {
            break;
        }

        // convert to polar coordinates
        let theta = acos(current.z / magnitude);
        let phi = atan2(current.y, current.x);
        magnitude_derivative = pow(magnitude, power - 1.0) * power * magnitude_derivative + 1.0;

        // scale and rotate the point
        let exp_magnitude = pow(magnitude, power);
        let exp_theta = theta * power;
        let exp_phi = phi * power;

        // convert back to cartesian coordinates
        current = exp_magnitude * Position(
            sin(exp_theta) * cos(exp_phi),
            sin(exp_phi) * sin(exp_theta),
            cos(exp_theta),
        );
        current += position;
    }
    let distance = 0.5 * log(magnitude) * magnitude / magnitude_derivative;
    return object(distance, colorize(position));
}

struct MarchResult {
    position: Position,
    distance: Distance,
    color: Color,
    closeness: Scalar,
    steps: u32,
}

fn march(start_position: Position, direction: Direction) -> MarchResult {
    var result: MarchResult;
    result.position = start_position;
    result.distance = -INFINITY;
    result.color = BACKGROUND_COLOR;
    var total_distance = Distance(0);
    var closeness = INFINITY;
    var iteration = 0u;
    for (; iteration < MAX_ITERATIONS && total_distance < MAX_TOTAL_DISTANCE; iteration++) {
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
    result.steps = iteration;
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
    const CAMERA_DIRECTION_Z = 1 / atan(FOV_DEGREES * PI / 180);
    let camera_direction = transform_direction(normalize(Direction(screen_position * parameters.aspect_scale, CAMERA_DIRECTION_Z)));
    let camera_position = transform_position(Position(0));
    let object_result = march(camera_position, camera_direction);
    var color = object_result.color;
    if (object_result.distance >= 0) {
        let object_position = object_result.position;
        let object_normal = calculate_normal(object_position);
        let to_sun = normalize(-SUN_DIRECTION);
        let to_camera = -camera_direction;
        let halfway = normalize(to_camera + to_sun);
        let specular = pow(max(dot(halfway, object_normal), 0), SPECULAR_SHARPNESS);
        let sun_result = march(object_position + object_normal * 2 * MIN_DISTANCE, to_sun);
        let ambient_occlusion = pow(1 - Scalar(object_result.steps) / Scalar(MAX_ITERATIONS), AMBIENT_OCCLUSION_SHARPNESS);
        color *= mix(AMBIENT_OCCLUSION_FACTOR, 1, ambient_occlusion);
        let shadow = Scalar(sun_result.distance < 0) * SHADOW_SHARPNESS * sun_result.closeness;
        color *= mix(SHADOW_FACTOR, 1, clamp(shadow, 0, 1));
        color += shadow * specular * SUN_COLOR;
    }
    return vec4(color, 1);
}
