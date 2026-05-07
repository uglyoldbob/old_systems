#[derive(Clone, Copy, Default, Debug)]
struct HdmiOutput {
    phactive: bool,
    hactive: bool,
    hsync: bool,
    hcount: u16,
    pvactive: bool,
    vactive: bool,
    vsync: bool,
    vcount: u16,
    de: bool,  // combinatorial: hactive & vactive
    pde: bool,
}

// 1280x720@60Hz timings (matching LiteX video_timings)
const H_ACTIVE: u16 = 1280;
const H_SYNC_OFFSET: u16 = 110;
const H_SYNC_WIDTH: u16 = 40;
const H_BLANKING: u16 = 370;

const V_ACTIVE: u16 = 720;
const V_SYNC_OFFSET: u16 = 5;
const V_SYNC_WIDTH: u16 = 5;
const V_BLANKING: u16 = 30;

const HSYNC_START: u16 = 6 + H_ACTIVE + H_SYNC_OFFSET;              // 1390
const HSYNC_END:   u16 = 6 + H_ACTIVE + H_SYNC_OFFSET + H_SYNC_WIDTH; // 1430
const HSCAN:       u16 = H_ACTIVE + H_BLANKING - 1;             // 1649

const VSYNC_START: u16 = 3 + V_ACTIVE + V_SYNC_OFFSET;              // 725
const VSYNC_END:   u16 = 3 + V_ACTIVE + V_SYNC_OFFSET + V_SYNC_WIDTH; // 730
const VSCAN:       u16 = V_ACTIVE + V_BLANKING - 1;             // 749

fn emit_image<F: Fn(&HdmiOutput) -> bool>(frame: &[HdmiOutput], name: &str, closure: F) {
    let mut image = Vec::new();
    for pixel in frame {
        if closure(pixel) {
            image.push(0xff);
            image.push(0);
            image.push(0);
        } else {
            image.push(0xff);
            image.push(0xff);
            image.push(0xff);
        }
    }

    use image::ImageEncoder;
    let mut f = std::fs::File::create(name).unwrap();
    let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut f);
    encoder.write_image(&image, 1650, 750, image::ExtendedColorType::Rgb8);
}

fn main() {
    println!("HDMI frame generator");

    let total_pixels = (HSCAN as usize + 1) * (VSCAN as usize + 1);
    let mut frame: Vec<HdmiOutput> = Vec::with_capacity(total_pixels);

    // Registered state (what the FSM holds)
    let mut hcount:  u16  = 0;
    let mut vcount:  u16  = 0;
    let mut hactive: bool = false;
    let mut vactive: bool = false;
    let mut phactive: bool = false;
    let mut pvactive: bool = false;
    let mut hsync:   bool = false;
    let mut vsync:   bool = false;

    for _ in 0..total_pixels {
        // Output current state (combinatorial DE)
        frame.push(HdmiOutput {
            hcount,
            vcount,
            hactive,
            vactive,
            hsync,
            vsync,
            phactive,
            pvactive,
            de: hactive && vactive,
            pde: phactive && pvactive,
        });

        // Now compute NextValue updates (all happen simultaneously, like hardware)
        let mut next_hcount  = hcount;
        let mut next_vcount  = vcount;
        let mut next_hactive = hactive;
        let mut next_vactive = vactive;
        let mut next_hsync   = hsync;
        let mut next_vsync   = vsync;
        let mut next_phactive = phactive;
        let mut next_pvactive = pvactive;

        // Horizontal counters/flags
        next_hcount = hcount + 1;
        if hcount == 0           { next_phactive = true;  }
        if hcount == 6           { next_hactive = true;  }
        if hcount == H_ACTIVE    { next_hactive = false; }
        if hcount == H_ACTIVE    { next_phactive = false; }
        if hcount == HSYNC_START { next_hsync   = true;  }
        if hcount == HSYNC_END   { next_hsync   = false; }
        if hcount == HSCAN       { next_hcount  = 0;     }

        // Vertical updates trigger at hsync_start
        if hcount == HSYNC_START {
            next_vcount = vcount + 1;

            if vcount == 0           { next_pvactive = true;  }
            if vcount == 3           { next_vactive = true;  }
            if vcount == V_ACTIVE    { next_vactive = false; }
            if vcount == V_ACTIVE    { next_pvactive = false; }
            if vcount == VSYNC_START { next_vsync   = true;  }
            if vcount == VSYNC_END   { next_vsync   = false; }
            if vcount == VSCAN       { next_vcount  = 0;     }
        }

        // Commit next state
        hcount  = next_hcount;
        vcount  = next_vcount;
        hactive = next_hactive;
        vactive = next_vactive;
        phactive = next_phactive;
        pvactive = next_pvactive;
        hsync   = next_hsync;
        vsync   = next_vsync;
    }

    emit_image(&frame, "hactive.bmp", |p| p.hactive);
    emit_image(&frame, "phactive.bmp", |p| p.phactive);
    emit_image(&frame, "hsync.bmp", |p| p.hsync);
    emit_image(&frame, "vactive.bmp", |p| p.vactive);
    emit_image(&frame, "pvactive.bmp", |p| p.pvactive);
    emit_image(&frame, "vsync.bmp", |p| p.vsync);
    emit_image(&frame, "de.bmp", |p| p.de);
    emit_image(&frame, "pde.bmp", |p| p.pde);
}