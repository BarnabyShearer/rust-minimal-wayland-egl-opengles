extern crate egli;
extern crate opengles;
extern crate wayland_client;
extern crate wayland_protocols;

use egli::Display as eglDisplay;
use opengles::glesv2;
use wayland_client::egl::WlEglSurface;
use wayland_client::protocol::wl_compositor;
use wayland_client::{Display, GlobalManager};
use wayland_protocols::xdg_shell::client::{xdg_surface, xdg_wm_base};

const WIDTH: i32 = 800;
const HEIGHT: i32 = 600;
const VERTEX_SRC: &[u8] = b"
    attribute vec4 vPosition;
    void main()
    {
        gl_Position = vPosition;
    }
";
const FRAGMENT_SRC: &[u8] = b"
    precision mediump float;
    void main()
    {
        gl_FragColor = vec4 ( 1.0, 0.0, 0.0, 1.0 );
    }
";

fn main() {
    // Create Wayland toplevel Window

    let (display, mut event_queue) =
        Display::connect_to_env().expect("Can't contact wayland display");
    let globals = GlobalManager::new(&display);
    // retrieve globals
    event_queue.sync_roundtrip().expect("Wayland issue");

    let compositor = globals
        .instantiate_exact::<wl_compositor::WlCompositor, _>(1, |comp| comp.implement_dummy())
        .expect("No compatible compositor");

    let surface = compositor
        .create_surface(|surface| surface.implement_dummy())
        .expect("Can't create surface");

    let shell = globals
        .instantiate_exact::<xdg_wm_base::XdgWmBase, _>(1, |shell| shell.implement_dummy())
        .expect("No compatible shell");

    let xdgs = shell
        .get_xdg_surface(&surface, |xdgs| {
            xdgs.implement_closure(
                |evt, xdgs| match evt {
                    xdg_surface::Event::Configure { serial } => {
                        xdgs.ack_configure(serial);
                    }
                    _ => unreachable!("Unkown shell event"),
                },
                (),
            )
        })
        .expect("Can't get xdg_surface");

    let _ = xdgs
        .get_toplevel(|toplevel| toplevel.implement_dummy())
        .expect("Can't set role toplevel");

    surface.commit();
    event_queue.sync_roundtrip().expect("Wayland issue");
    surface.commit();

    // Use EGL to setup OpenGL ES 2.0

    let egl_display = eglDisplay::from_display_id(display.get_display_ptr() as *mut _)
        .expect("Failed to get EGL display");

    egl_display.initialize().expect("Failed to initialize");

    let config = *egl_display
        .config_filter()
        .choose_configs()
        .expect("Failed to get configurations")
        .first()
        .expect("Mo compatible EGL configuration was found");

    let gl = WlEglSurface::new(&surface, WIDTH, HEIGHT);

    let egl_surface = egl_display
        .create_window_surface(config, gl.ptr() as *mut _)
        .expect("Failed to create window surface");

    let egl_context = egl_display
        .create_context_with_client_version(config, egli::ContextClientVersion::OpenGlEs2)
        .expect("Failed to create OpenGL context");

    egl_display
        .make_current(&egl_surface, &egl_surface, &egl_context)
        .expect("Make current failed");

    // Background

    glesv2::clear_color(0.0f32, 0.3f32, 0.3f32, 0.0f32);
    glesv2::clear(glesv2::GL_COLOR_BUFFER_BIT);

    // Hook up shaders

    let program = glesv2::create_program();
    let add_shader = |shader_type, src| {
        let shader = glesv2::create_shader(shader_type);
        if shader == 0 {
            println!("Can't create shader");
        }
        glesv2::shader_source(shader, src);
        glesv2::compile_shader(shader);
        if glesv2::get_shaderiv(shader, glesv2::GL_COMPILE_STATUS) != glesv2::GL_TRUE as i32 {
            println!(
                "Shader issue: {}",
                glesv2::get_shader_info_log(shader, 1024).expect("Log err")
            );
        }
        glesv2::attach_shader(program, shader);
    };
    add_shader(glesv2::GL_VERTEX_SHADER, VERTEX_SRC);
    add_shader(glesv2::GL_FRAGMENT_SHADER, FRAGMENT_SRC);
    glesv2::bind_attrib_location(program, 0, "vPosition");
    glesv2::link_program(program);
    if glesv2::get_programiv(program, glesv2::GL_LINK_STATUS) != glesv2::GL_TRUE as i32 {
        println!(
            "Link issue: {}",
            glesv2::get_program_info_log(program, 1024).expect("Log err")
        );
    }
    glesv2::use_program(program);

    // Draw triangle

    glesv2::vertex_attrib_pointer(
        0,
        3,
        glesv2::GL_FLOAT,
        false,
        0,
        &[
            0.0f32, 0.5f32, 0.0f32, /* */
            -0.5f32, -0.5f32, 0.0f32, /* */
            0.5f32, -0.5f32, 0.0f32, /* */
        ],
    );
    glesv2::enable_vertex_attrib_array(0);
    glesv2::draw_arrays(glesv2::GL_TRIANGLES, 0, 3);

    // Display

    egl_display
        .swap_buffers(&egl_surface)
        .expect("Failed to swap buffers");

    // Wayland event look

    loop {
        event_queue.dispatch().expect("Wayland issue");
    }
}
