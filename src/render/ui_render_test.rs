//! The retained UI examples drawn offscreen: the trees `examples/ui_shop.rs`
//! and `examples/ui_panel.rs` build with the builders, layout presets and
//! names give the same pixels, state by state, as the trees those examples
//! built before with `add_child`, raw layout styles and setters.
//!
//! The pictures are real: the example files themselves are included, their
//! trees go through the engine's UI drawing, and its sprites are drawn with
//! the sprite shader, pipeline and pass the window uses, into a texture read
//! back here. Only the window is missing. Skips without a GPU (a skip is not
//! a pass).
//!
//! One thing differs from the picture the old shop showed in its day: its
//! dialog. The old tree asks for it in the middle with `auto` side margins,
//! which did nothing on a screen root then, so it sat on the window's left
//! edge. Roots take their auto margins now, so the old tree draws the dialog
//! where it meant to, and the example's `UiLayout::centered(..).top(..)`
//! puts it on the same pixels.
use super::batch::SpriteInstance;
use super::gpu::{GpuTexture, Renderer, SpritePass, sprite_pipeline};
use super::render_test_support::{Image, TestGpu, gpu, output_dir, target};
use crate::text::Text;
use crate::ui::Ui;
use crate::ui::UiEvent;
use crate::ui::UiId;
use crate::ui::UiInput;
use crate::{Color, Ctx, Vec2, d3::UiPanel as UiPanel3D, vec2};

#[allow(dead_code)]
#[path = "../../examples/ui_shop.rs"]
mod shop;

#[allow(dead_code)]
#[path = "../../examples/ui_panel.rs"]
mod panel;

/// The trees and event code the examples had before the builders, kept as
/// they were (only `ctx.ui_retained` is `ui` here).
mod before {
    use crate::ui::layout::prelude::*;
    #[allow(unused_imports)]
    use ponko::prelude::*;
    #[allow(unused_imports)]
    use ponko::ui::{Ui, UiEvent, UiId, UiInput, UiLayout, UiPaint, UiWidget};

    fn fixed(width: f32, height: f32) -> Style {
        Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            flex_shrink: 0.0,
            ..Default::default()
        }
    }

    /// What the old shop kept in its closure.
    pub struct Shop {
        pub title: UiId,
        pub items: Vec<UiId>,
        pub buy: UiId,
        pub dialog: UiId,
        pub message: UiId,
        pub confirm: UiId,
        pub cancel: UiId,
        pub selected: Option<usize>,
        // Not kept by the old code; the test aims the pointer with them.
        pub note: UiId,
        pub gift: UiId,
    }

    pub fn shop(ui: &mut Ui) -> Shop {
        let root = ui.add(
            UiWidget::Panel,
            Style {
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                flex_direction: FlexDirection::Column,
                padding: crate::ui::layout::Rect::length(24.0),
                gap: Size::length(12.0),
                ..Default::default()
            },
        );
        let title = ui.add_child(root, UiWidget::Label, fixed(400.0, 32.0));
        ui.set_text(title, "General store — choose an item");
        let note = ui.add_child(root, UiWidget::TextInput, fixed(360.0, 36.0));
        ui.set_text(note, "Your order note");
        let scroll = ui.add_child(
            root,
            UiWidget::Scroll,
            Style {
                size: Size {
                    width: percent(1.0),
                    height: length(360.0),
                },
                min_size: Size::length(0.0),
                flex_shrink: 1.0,
                ..Default::default()
            },
        );
        let grid = ui.add_child(
            scroll,
            UiWidget::Panel,
            Style {
                display: Display::Grid,
                size: Size {
                    width: percent(0.95),
                    height: auto(),
                },
                grid_template_columns: vec![fr(1.0), fr(1.0), fr(1.0)],
                grid_auto_rows: vec![length(76.0)],
                gap: Size::length(8.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
        );
        let mut items = Vec::new();
        for i in 0..36 {
            let item = ui.add_child(
                grid,
                UiWidget::Button,
                Style {
                    min_size: Size::length(0.0),
                    padding: crate::ui::layout::Rect::length(8.0),
                    ..Default::default()
                },
            );
            ui.set_text(item, format!("Item {}\n{} coins", i + 1, (i + 1) * 3));
            ui.set_paint(
                item,
                UiPaint {
                    wrap: true,
                    border_width: 1.0,
                    ..Default::default()
                },
            );
            items.push(item);
        }
        let gift = ui.add_child(root, UiWidget::Toggle, fixed(220.0, 28.0));
        ui.set_text(gift, "Gift wrap");
        let buy = ui.add_child(root, UiWidget::Button, fixed(220.0, 40.0));
        ui.set_text(buy, "Buy selected item");
        ui.set_enabled(buy, false);
        ui.push_screen(root, false);

        let dialog = ui.add(
            UiWidget::Panel,
            Style {
                size: Size {
                    width: length(360.0),
                    height: length(180.0),
                },
                margin: crate::ui::layout::Rect {
                    left: auto(),
                    right: auto(),
                    top: length(180.0),
                    bottom: auto(),
                },
                padding: crate::ui::layout::Rect::length(20.0),
                flex_direction: FlexDirection::Column,
                gap: Size::length(12.0),
                ..Default::default()
            },
        );
        let message = ui.add_child(dialog, UiWidget::Label, fixed(320.0, 40.0));
        let confirm = ui.add_child(dialog, UiWidget::Button, fixed(240.0, 36.0));
        ui.set_text(confirm, "Confirm purchase");
        let cancel = ui.add_child(dialog, UiWidget::Button, fixed(240.0, 30.0));
        ui.set_text(cancel, "Back");
        Shop {
            title,
            items,
            buy,
            dialog,
            message,
            confirm,
            cancel,
            selected: None,
            note,
            gift,
        }
    }

    pub fn shop_events(ui: &mut Ui, shop: &mut Shop) {
        let mut clicked = None;
        for event in ui.events() {
            if let UiEvent::Clicked(id) = event {
                clicked = Some(id);
            }
        }
        if let Some(id) = clicked {
            if let Some(index) = shop.items.iter().position(|&item| item == id) {
                shop.selected = Some(index);
                ui.set_enabled(shop.buy, true);
                ui.set_text(shop.buy, format!("Buy item {}", index + 1));
            } else if id == shop.buy {
                ui.set_text(shop.message, format!("Buy item {}?", shop.selected.unwrap() + 1));
                ui.push_screen(shop.dialog, true);
            } else if id == shop.confirm || id == shop.cancel {
                ui.pop_screen();
                if id == shop.confirm {
                    ui.set_text(shop.title, "Purchase complete!");
                }
            }
        }
    }

    /// The old terminal's tree; returns its button.
    pub fn panel(ui: &mut Ui) -> UiId {
        let root = ui.add(
            UiWidget::Panel,
            Style {
                size: Size {
                    width: percent(1.0),
                    height: percent(1.0),
                },
                flex_direction: FlexDirection::Column,
                padding: crate::ui::layout::Rect::length(20.0),
                gap: Size::length(12.0),
                ..Default::default()
            },
        );
        let buy = ui.add_child(
            root,
            UiWidget::Button,
            Style {
                size: Size {
                    width: length(200.0),
                    height: length(60.0),
                },
                flex_shrink: 0.0,
                ..Default::default()
            },
        );
        ui.set_text(buy, "Buy");
        ui.push_screen(root, false);
        buy
    }
}

/// The window's sprite pipeline on a device with no window.
struct Painter {
    gpu: TestGpu,
    pipeline: wgpu::RenderPipeline,
    camera_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

impl Painter {
    /// `None` without a GPU.
    fn new() -> Option<Self> {
        let gpu = gpu()?;
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::include_wgsl!("sprite.wgsl"));
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(1, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        // The window's sampler for sprites and text.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Some(Self {
            pipeline: sprite_pipeline(device, &layout, &shader, FORMAT),
            camera_layout,
            texture_layout,
            sampler,
            gpu,
        })
    }

    /// A texture of `side` x `side` sprites can draw, with `rgba` or empty.
    fn texture(&self, side: u32, rgba: Option<&[u8]>) -> GpuTexture {
        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: side,
                height: side,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        if let Some(rgba) = rgba {
            self.write(&texture, [0, 0], [side, side], rgba);
        }
        let view = texture.create_view(&Default::default());
        GpuTexture {
            bind_group: self.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.texture_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }),
            texture,
            nearest: false,
            mips: None,
        }
    }

    fn write(&self, texture: &wgpu::Texture, at: [u32; 2], size: [u32; 2], rgba: &[u8]) {
        self.gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: at[0],
                    y: at[1],
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size[0]),
                rows_per_image: Some(size[1]),
            },
            wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
        );
    }

    /// A screen to draw UI on: a `Ctx` with no window and no renderer, and
    /// the two textures its UI draws with (white, the glyph atlas).
    fn screen(&self, size: Vec2) -> Screen {
        let mut ctx = Ctx::new(1.0 / 60.0, size);
        ctx.gfx.stand_in_textures();
        Screen {
            ctx,
            textures: vec![self.texture(1, Some(&[255; 4])), self.texture(Text::ATLAS_SIZE, None)],
        }
    }

    /// The picture of what `screen`'s `Ctx` has drawn into its UI list
    /// since the last one, `size` pixels, on the window's clear colour.
    fn shot(&self, screen: &mut Screen, size: Vec2) -> Image {
        use wgpu::util::DeviceExt;
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        // Glyphs made while drawing go into the atlas, as the renderer does.
        for (at, glyph, rgba) in screen.ctx.gfx.text.uploads.drain(..) {
            self.write(&screen.textures[1].texture, at, glyph, &rgba);
        }
        let (sprites, batches) = screen.ctx.gfx.ui_list.finish();
        assert!(!sprites.is_empty(), "the UI drew nothing");
        let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice::<SpriteInstance, u8>(sprites),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let camera = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&crate::ctx::screen_projection(size).to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let camera = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let output = target(device, (size.x as u32, size.y as u32), FORMAT);
        let view = output.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        Renderer::sprite_pass(
            &mut encoder,
            &view,
            Some(Color::BLACK),
            None,
            &SpritePass {
                viewport: [size.x as u32, size.y as u32],
                scale: 1.0,
                pipeline: &self.pipeline,
                instances: &instances,
                textures: &screen.textures,
                layers: &[(&camera, batches, 0)],
            },
        );
        queue.submit([encoder.finish()]);
        let image = Image::read(device, queue, &output);
        screen.ctx.gfx.ui_list.clear();
        image
    }
}

/// See [`Painter::screen`].
struct Screen {
    ctx: Ctx,
    textures: Vec<GpuTexture>,
}

/// One frame of pointer input.
fn pointer(at: Vec2, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: at,
        pressed,
        held: pressed,
        released,
        ..Default::default()
    }
}

/// Saves both pictures of a failing step to look at, and says where.
fn keep(step: &str, before: &Image, after: &Image) -> String {
    let dir = output_dir();
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("creating {}: {e}", dir.display()));
    let name = format!("ui_{}", step.replace(' ', "_"));
    for (side, image) in [("before", before), ("after", after)] {
        let path = dir.join(format!("{name}_{side}.png"));
        image::save_buffer(&path, &image.rgba, image.width, image.height, image::ColorType::Rgba8)
            .unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
    }
    format!("{}/{name}_{{before,after}}.png", dir.display())
}

/// Asserts two pictures are the same, pixel for pixel, and not blank. Only
/// a failing step writes its pictures (to the render tests' output folder).
fn assert_same(step: &str, before: &Image, after: &Image) {
    let colours: std::collections::HashSet<[u8; 4]> = after.rgba.as_chunks::<4>().0.iter().copied().collect();
    if colours.len() <= 8 {
        let saved = keep(step, before, after);
        panic!("{step}: the picture is blank ({} colours); see {saved}", colours.len());
    }
    let different = changed(before, after);
    if different != 0 {
        let saved = keep(step, before, after);
        panic!("{step}: the builders' picture differs from the old tree's in {different} pixels; see {saved}");
    }
}

/// How many pixels differ between two pictures.
fn changed(a: &Image, b: &Image) -> usize {
    let (a, b) = (a.rgba.as_chunks::<4>().0, b.rgba.as_chunks::<4>().0);
    a.iter().zip(b).filter(|(a, b)| a != b).count()
}

#[test]
fn the_shop_example_draws_what_its_hand_built_tree_drew() {
    let Some(painter) = Painter::new() else { return };
    let size = vec2(900.0, 700.0);
    let (mut old, mut new) = (painter.screen(size), painter.screen(size));
    let mut ids = before::shop(&mut old.ctx.ui);
    shop::build(&mut new.ctx);

    // One frame of input for both, each example's own event code, a picture
    // of each.
    let mut pictures: Vec<Image> = Vec::new();
    let mut step = |name: &str, old: &mut Screen, new: &mut Screen, ids: &mut before::Shop, input: UiInput| {
        old.ctx.ui.frame(size, &input);
        before::shop_events(&mut old.ctx.ui, ids);
        old.ctx.draw_retained_ui();
        new.ctx.ui.frame(size, &input);
        shop::shop(&mut new.ctx);
        new.ctx.draw_retained_ui();
        let (before, after) = (painter.shot(old, size), painter.shot(new, size));
        assert_same(&format!("shop {name}"), &before, &after);
        pictures.push(after);
    };
    let away = vec2(880.0, 690.0);
    step("as built", &mut old, &mut new, &mut ids, pointer(away, false, false));

    // Every node the old code kept an id for is where the name finds it.
    let center = |ui: &Ui, id: UiId| ui.rect(id).unwrap().center();
    let item = center(&old.ctx.ui, ids.items[4]);
    assert_eq!(item, center(&new.ctx.ui, new.ctx.ui.find("item.5").unwrap()));
    step(
        "hovering an item",
        &mut old,
        &mut new,
        &mut ids,
        pointer(item, false, false),
    );
    step(
        "scrolled",
        &mut old,
        &mut new,
        &mut ids,
        UiInput {
            wheel: vec2(0.0, 120.0),
            ..pointer(item, false, false)
        },
    );
    // After the scroll another item is under the pointer; click the fifth.
    let item = center(&old.ctx.ui, ids.items[4]);
    step("item pressed", &mut old, &mut new, &mut ids, pointer(item, true, false));
    step("item chosen", &mut old, &mut new, &mut ids, pointer(item, false, true));
    assert_eq!(new.ctx.ui.text(new.ctx.ui.find("buy").unwrap()), Some("Buy item 5"));
    let buy = center(&old.ctx.ui, ids.buy);
    step("buy pressed", &mut old, &mut new, &mut ids, pointer(buy, true, false));
    step("dialog open", &mut old, &mut new, &mut ids, pointer(buy, false, true));
    assert_eq!(
        new.ctx.ui.text(new.ctx.ui.find("message").unwrap()),
        Some("Buy item 5?")
    );
    // The dialog is where the old tree's margins ask for it: centered, 180
    // pixels down.
    let dialog = new.ctx.ui.find("dialog").unwrap();
    assert_eq!(new.ctx.ui.rect(dialog), old.ctx.ui.rect(ids.dialog));
    assert_eq!(new.ctx.ui.rect(dialog).unwrap().min, vec2(270.0, 180.0));
    // Back closes the dialog and buys nothing.
    let cancel = center(&old.ctx.ui, ids.cancel);
    assert_eq!(cancel, center(&new.ctx.ui, new.ctx.ui.find("cancel").unwrap()));
    step(
        "back pressed",
        &mut old,
        &mut new,
        &mut ids,
        pointer(cancel, true, false),
    );
    step("backed out", &mut old, &mut new, &mut ids, pointer(cancel, false, true));
    assert_eq!(
        new.ctx.ui.text(new.ctx.ui.find("title").unwrap()),
        Some("General store — choose an item")
    );
    step(
        "buy pressed again",
        &mut old,
        &mut new,
        &mut ids,
        pointer(buy, true, false),
    );
    step(
        "dialog open again",
        &mut old,
        &mut new,
        &mut ids,
        pointer(buy, false, true),
    );
    let confirm = center(&old.ctx.ui, ids.confirm);
    step(
        "confirm pressed",
        &mut old,
        &mut new,
        &mut ids,
        pointer(confirm, true, false),
    );
    step("bought", &mut old, &mut new, &mut ids, pointer(confirm, false, true));
    assert_eq!(
        new.ctx.ui.text(new.ctx.ui.find("title").unwrap()),
        Some("Purchase complete!")
    );

    // The gift-wrap toggle turns on.
    let gift = new.ctx.ui.find("gift").unwrap();
    let toggle = center(&old.ctx.ui, ids.gift);
    assert_eq!(toggle, center(&new.ctx.ui, gift));
    step(
        "toggle pressed",
        &mut old,
        &mut new,
        &mut ids,
        pointer(toggle, true, false),
    );
    step(
        "gift wrapped",
        &mut old,
        &mut new,
        &mut ids,
        pointer(toggle, false, true),
    );
    assert_eq!(new.ctx.ui.value(gift), Some(1.0));
    assert_eq!(new.ctx.ui.value(gift), old.ctx.ui.value(ids.gift));

    // Typing into the order note: a click focuses it, then the keys arrive.
    let note = new.ctx.ui.find("note").unwrap();
    let field = center(&old.ctx.ui, ids.note);
    assert_eq!(field, center(&new.ctx.ui, note));
    step(
        "note pressed",
        &mut old,
        &mut new,
        &mut ids,
        pointer(field, true, false),
    );
    step(
        "note focused",
        &mut old,
        &mut new,
        &mut ids,
        pointer(field, false, true),
    );
    step(
        "typed",
        &mut old,
        &mut new,
        &mut ids,
        UiInput {
            text: ", please".into(),
            ..pointer(field, false, false)
        },
    );
    let typed = new.ctx.ui.text(note).unwrap().to_owned();
    assert!(typed.contains(", please"), "{typed:?}");
    assert_eq!(Some(typed.as_str()), old.ctx.ui.text(ids.note));

    // The comparison can fail: every step changed the picture.
    for pair in pictures.windows(2) {
        assert!(
            changed(&pair[0], &pair[1]) > 0,
            "a step that changed nothing compares nothing"
        );
    }
    // The modal dialog darkens the screen behind it; popping it, with Back
    // or with Confirm, lights it again. (An empty spot of the shop's panel,
    // right of the buy button.)
    let lit = |i: usize| pictures[i].pixel(600, 600)[0];
    let [pressed, open, backed, open_again, bought] = [5, 6, 8, 10, 12].map(lit);
    assert!(
        open < pressed && backed == pressed && open_again == open && bought == pressed,
        "{pressed} {open} {backed} {open_again} {bought}"
    );
}

#[test]
fn the_panel_example_draws_what_its_hand_built_tree_drew() {
    let Some(painter) = Painter::new() else { return };
    // The panel's pixels: 600 x 400, whatever the window is.
    let size = vec2(600.0, 400.0);
    let (mut old, mut new) = (painter.screen(size), painter.screen(size));
    let mut old_ui = Ui::default();
    let old_buy = before::panel(&mut old_ui);
    // The example's setup makes a scene with the panel in it, with no window.
    let mut game = Ctx::new(1.0 / 60.0, vec2(960.0, 640.0));
    game.set_headless(true);
    panel::setup(&mut game);
    assert_eq!(game.world.query::<&UiPanel3D>().count(), 1);

    let mut pictures: Vec<Image> = Vec::new();
    let mut step = |name: &str, old_ui: &mut Ui, game: &mut Ctx, input: UiInput| {
        old_ui.frame(size, &input);
        while let Some(event) = old_ui.next_event() {
            if event == UiEvent::Clicked(old_buy) {
                old_ui.set_text(old_buy, "Bought!");
            }
        }
        old.ctx.draw_ui_tree(old_ui, size);
        // The panel's tree gets the same input, then the example's system.
        game.world
            .query::<&mut UiPanel3D>()
            .for_each(|p| p.ui.frame(size, &input));
        panel::terminal(game);
        let mut ui = Ui::default();
        game.world
            .query::<&mut UiPanel3D>()
            .for_each(|p| std::mem::swap(&mut p.ui, &mut ui));
        new.ctx.draw_ui_tree(&mut ui, size);
        game.world
            .query::<&mut UiPanel3D>()
            .for_each(|p| std::mem::swap(&mut p.ui, &mut ui));
        let (before, after) = (painter.shot(&mut old, size), painter.shot(&mut new, size));
        assert_same(&format!("panel {name}"), &before, &after);
        pictures.push(after);
    };
    let button = vec2(120.0, 50.0);
    step(
        "as built",
        &mut old_ui,
        &mut game,
        pointer(vec2(590.0, 390.0), false, false),
    );
    step(
        "hovering the button",
        &mut old_ui,
        &mut game,
        pointer(button, false, false),
    );
    step("pressed", &mut old_ui, &mut game, pointer(button, true, false));
    step("bought", &mut old_ui, &mut game, pointer(button, false, true));
    assert_eq!(old_ui.text(old_buy), Some("Bought!"));
    for pair in pictures.windows(2) {
        assert!(
            changed(&pair[0], &pair[1]) > 0,
            "a step that changed nothing compares nothing"
        );
    }
}
