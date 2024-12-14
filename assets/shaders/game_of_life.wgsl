
@group(0) @binding(0) var read_texture: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var write_texture: texture_storage_2d<rgba8unorm, write>;

fn is_alive(location: vec2<i32>, offset_x: i32, offset_y: i32) -> i32 {
    let value: vec4<f32> = textureLoad(read_texture, location + vec2<i32>(offset_x, offset_y));
    return i32(value.x);
}

fn do_2d(location: vec2<i32>) {
    let n_alive =  
        is_alive(location, -1, -1) +
        is_alive(location, -1,  0) +
        is_alive(location, -1,  1) +
        is_alive(location,  0, -1) +
        is_alive(location,  0,  1) +
        is_alive(location,  1, -1) +
        is_alive(location,  1,  0) +
        is_alive(location,  1,  1);
    var alive: bool;
    if (n_alive == 3) {
        alive = true;
    } else if (n_alive == 2) {
        let currently_alive = is_alive(location, 0, 0);
        alive = bool(currently_alive);
    } else {
        alive = false;
    }
    var color: vec4<f32> = textureLoad(read_texture, location);
    if alive {
        color = vec4(1.0);
    } else {
        color = color - 0.01; 
    }

    storageBarrier();

    textureStore(write_texture, location, color);
}


fn do_1d(location: vec2<i32>){
    let idx = is_alive(location,-1,0) * 4 + is_alive(location,0,0) * 2 + is_alive(location,1,0);
    var color: vec4<f32>;
    if idx == 1 | idx == 2 | idx == 3 | idx == 4 {
        color = vec4(1.0);
    } else {
        color = vec4(0.0);
    }
    storageBarrier();
    textureStore(write_texture, location, color);
}
    

@compute @workgroup_size(8, 8, 1)
fn update(@builtin(global_invocation_id) invocation_id: vec3<u32>) {

    let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));
    if invocation_id.y == 0{
        do_1d(location);
    } else if invocation_id.y < 100 {
        var color: vec4<f32> = textureLoad(read_texture, location + vec2(0,-1));
        storageBarrier();
        textureStore(write_texture, location, color);
    } else {
        do_2d(location);
    }
}
