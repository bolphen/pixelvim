use crate::color::Color;
use crate::engine::Buffer;
use crate::error::Error;
use crate::image::Image;
use mlua::prelude::{IntoLua, *};
use std::collections::HashMap;

pub struct LuaInstance {
    lua: Lua,
}
impl From<mlua::Error> for Error {
    fn from(value: mlua::Error) -> Self {
        Error::LuaError(value.to_string())
    }
}

impl LuaUserData for Color {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("r", |_, this| Ok(this.0));
        fields.add_field_method_get("g", |_, this| Ok(this.1));
        fields.add_field_method_get("b", |_, this| Ok(this.2));
        fields.add_field_method_get("a", |_, this| Ok(this.3));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("toLinear", |lua, this, srgb: bool| -> Result<LuaTable, _> {
            lua.create_table_from((1..=4).zip(this.to_linear(srgb)))
        });
        methods.add_meta_method("__eq", |_lua, this, other: Color| -> Result<LuaValue, _> {
            Ok(LuaValue::Boolean(*this == other))
        });
    }
}

impl FromLua for Color {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match &value {
            LuaValue::UserData(value) => {
                if value.is::<Color>() {
                    return Ok(*value.borrow::<Color>()?);
                }
            }
            LuaValue::Table(table) => {
                let r: u8 = table.get("r")?;
                let g: u8 = table.get("g")?;
                let b: u8 = table.get("b")?;
                let a: u8 = table.get("a")?;
                return Ok((r, g, b, a).into());
            }
            _ => (),
        }
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "Color".into(),
            message: None,
        })
    }
}

impl LuaUserData for Image {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.width()));
        fields.add_field_method_get("height", |_, this| Ok(this.height()));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "get",
            |lua, this, (x, y): (i32, i32)| -> Result<LuaValue, _> {
                Ok(if let Some(color) = this.get(x - 1, y - 1) {
                    color.into_lua(lua)?
                } else {
                    LuaNil
                })
            },
        );
        methods.add_method_mut(
            "set",
            |_, this, (x, y, r, g, b, a): (i32, i32, u8, u8, u8, u8)| -> Result<(), _> {
                this.get_mut(x - 1, y - 1).map(|c| *c = Color(r, g, b, a));
                Ok(())
            },
        );
        methods.add_method_mut(
            "setLinear",
            |_,
             this,
             (x, y, r, g, b, a, srgb): (i32, i32, f32, f32, f32, f32, bool)|
             -> Result<(), _> {
                this.get_mut(x - 1, y - 1)
                    .map(|c| *c = Color::from_linear([r, g, b, a], srgb));
                Ok(())
            },
        );
        methods.add_method("clone", |lua, this, ()| -> Result<LuaValue, _> {
            this.clone().into_lua(lua)
        });
        methods.add_method("blank", |lua, this, ()| -> Result<LuaValue, _> {
            this.blank().into_lua(lua)
        });
        methods.add_function(
            "new",
            |lua, (width, height): (usize, usize)| -> Result<LuaValue, _> {
                Image::new_with(width, height, Color(0, 0, 0, 0)).into_lua(lua)
            },
        );
    }
}

impl FromLua for Image {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match &value {
            LuaValue::UserData(value) => {
                if value.is::<Image>() {
                    return Ok(value.borrow::<Image>()?.clone());
                }
            }
            LuaValue::Table(table) => {
                let width: usize = table.get("width")?;
                let height: usize = table.get("height")?;
                let data: LuaTable = table.get("data")?;
                let mut bytes: Vec<u8> = Vec::new();
                for d in data.sequence_values::<u8>() {
                    bytes.push(d?);
                }
                if bytes.len() == 4 * width * height {
                    return Ok(Image::new_from_bytes(width, height, bytes));
                }
            }
            _ => (),
        }
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "Image".into(),
            message: None,
        })
    }
}

struct BufferSnapshot {
    width: usize,
    height: usize,
    num_layers: usize,
    num_frames: usize,
    current_layer: usize,
    current_frame: usize,
    images: Vec<Vec<Image>>,
}

impl From<&Buffer> for BufferSnapshot {
    fn from(value: &Buffer) -> Self {
        let num_layers = value.num_layers();
        let num_frames = value.num_frames();
        BufferSnapshot {
            width: value.width(),
            height: value.height(),
            num_layers,
            num_frames,
            current_layer: value.session.current_layer,
            current_frame: value.session.current_frame,
            images: value.session.current_images().clone(),
        }
    }
}
impl LuaUserData for BufferSnapshot {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.width));
        fields.add_field_method_get("height", |_, this| Ok(this.height));
        fields.add_field_method_get("layerCount", |_, this| Ok(this.num_layers));
        fields.add_field_method_get("frameCount", |_, this| Ok(this.num_frames));
        fields.add_field_method_get("currentLayer", |_, this| Ok(this.current_layer + 1));
        fields.add_field_method_get("currentFrame", |_, this| Ok(this.current_frame + 1));
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "get",
            |lua, this, (x, y): (usize, usize)| -> Result<LuaValue, _> {
                if x == 0 || y == 0 {
                    return Ok(LuaNil);
                }
                if let Some(image) = this.images.get(x - 1).and_then(|l| l.get(y - 1)) {
                    Ok(image.clone().into_lua(lua)?)
                } else {
                    Ok(LuaNil)
                }
            },
        );
        methods.add_method("getCurrent", |lua, this, ()| -> Result<LuaValue, _> {
            this.images[this.current_layer][this.current_frame]
                .clone()
                .into_lua(lua)
        });
    }
}

#[derive(Clone)]
pub struct OutputData {
    current_layer: usize,
    current_frame: usize,
    pub name: Option<String>,
    pub changed_frames: HashMap<(usize, usize), Image>,
}

impl LuaUserData for OutputData {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_set("name", |_, this, name| {
            this.name = name;
            Ok(())
        });
    }
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "change",
            |_, this, (l, f, image): (usize, usize, Image)| -> Result<(), _> {
                let l = l.checked_sub(1).ok_or(LuaError::runtime("Invalid index"))?;
                let f = f.checked_sub(1).ok_or(LuaError::runtime("Invalid index"))?;
                this.changed_frames.insert((l, f), image);
                Ok(())
            },
        );
        methods.add_method_mut("changeCurrent", |_, this, image: Image| -> Result<(), _> {
            this.changed_frames
                .insert((this.current_layer, this.current_frame), image);
            Ok(())
        });
    }
}

impl FromLua for OutputData {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        if let LuaValue::UserData(value) = &value {
            if value.is::<OutputData>() {
                return Ok(value.borrow::<OutputData>()?.clone());
            }
        }
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "Output".into(),
            message: None,
        })
    }
}
pub type LuaHandle = std::thread::JoinHandle<Result<LuaInstance, LuaError>>;
impl LuaInstance {
    pub fn new() -> Result<Self, Error> {
        Ok(LuaInstance { lua: Lua::new() })
    }
    pub fn set_modifier(&self, value: Option<i32>) -> Result<(), Error> {
        let table = self.lua.create_table()?;
        if let Some(value) = value {
            table.set("value", value)?;
        }
        self.lua.globals().set("MODIFIER", table)?;
        Ok(())
    }
    pub fn init(&self, buffer: &Buffer, color: Color) -> Result<(), Error> {
        let snapshot: BufferSnapshot = buffer.into();
        let current_layer = snapshot.current_layer;
        let current_frame = snapshot.current_frame;
        self.lua
            .globals()
            .set("INPUT", snapshot.into_lua(&self.lua)?)?;
        self.lua
            .globals()
            .set("COLOR", color.into_lua(&self.lua)?)?;
        self.lua.globals().set(
            "OUTPUT",
            OutputData {
                current_layer,
                current_frame,
                name: None,
                changed_frames: HashMap::new(),
            }
            .into_lua(&self.lua)?,
        )?;
        Ok(())
    }
    pub fn exec(self, code: String) -> (LuaHandle, std::sync::mpsc::Sender<bool>) {
        let (snd, rcv) = std::sync::mpsc::channel::<bool>();
        self.lua.set_hook(
            mlua::HookTriggers::new().every_nth_instruction(10000),
            move |lua, _debug| {
                if rcv.try_recv().is_ok_and(|v| v) {
                    lua.remove_hook();
                    return Err(LuaError::runtime("Cancelled"));
                }
                Ok(mlua::VmState::Continue)
            },
        );
        (
            // spawn a thread and pass the lua instance to it
            // then we go into "running" mode
            // either we get back the lua instance upon completion, or the thread is interrupted
            // before that via the interrupt signal sender
            std::thread::spawn(move || self.lua.load(code).exec().map(|_| self)),
            snd,
        )
    }
    pub fn retrieve_output(&self) -> Result<OutputData, Error> {
        Ok(self.lua.globals().get("OUTPUT")?)
    }
}
