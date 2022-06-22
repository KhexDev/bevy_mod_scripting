use bevy::prelude::*;
use bevy_console::ConsolePlugin;
use bevy_event_priority::PriorityEventWriter;
use bevy_mod_scripting::{
    APIProvider, AddScriptHost, AddScriptHostHandler, LuaEvent, LuaFile, RLuaScriptHost,
    Recipients, Script, ScriptCollection, ScriptingPlugin, ScriptError, AddScriptApiProvider, LuaDocFragment, GenDocumentation,
};
use rand::prelude::SliceRandom;
use tealr::mlu::mlua::Value;
use tealr::{TypeWalker, TypeName};
use tealr::mlu::TypedFunction;
use tealr::mlu::{mlua,mlua::{Lua, ToLua},TealData};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{atomic::AtomicU32, Mutex};

#[derive(Clone)]
pub struct MyLuaArg;

impl<'lua> ToLua<'lua> for MyLuaArg {
    fn to_lua(self, lua: &'lua Lua) -> tealr::mlu::mlua::Result<tealr::mlu::mlua::Value<'lua>> {
        Ok(Value::Nil)
    }
}





#[derive(Clone,tealr::MluaUserData, TypeName)]
/// This is acts as a documentation and function holder
/// We can add some general documentation about what it holds
/// but also specific function level documenation
pub struct APIModule;


impl TealData for APIModule {
    fn add_methods<'lua, T: tealr::mlu::TealDataMethods<'lua, Self>>(methods: &mut T) {

        methods.document_type("This is module level documentation for our api, it will be shown first");
        methods.document_type("");

        methods.document("Here we document the next function");
        methods.document("## Markdown!:");
        methods.document(
            "```lua
                local hello = \"string\"  
            \n```");
        methods.add_function("my_function", |_,()| Ok("hello world!"));

        methods.generate_help();
    }

}


/// This is tealr's way to export global items
/// Here `my_api` will be available globally in the lua script
struct Export;
impl tealr::mlu::ExportInstances for Export {
    fn add_instances<'lua, T: tealr::mlu::InstanceCollector<'lua>>(instance_collector: &mut T) -> mlua::Result<()> {
        instance_collector.add_instance("my_api".into(), |_| {
            Ok(APIModule)
        })

    }
}


#[derive(Default)]
pub struct LuaAPIProvider;

/// the custom Lua api, world is provided via a global pointer,
/// and callbacks are defined only once at script creation
impl APIProvider for LuaAPIProvider {
    type Target = Mutex<Lua>;
    type DocTarget = LuaDocFragment;

    fn attach_api(&mut self,ctx: &mut Self::Target) -> Result<(),ScriptError> {
       // callbacks can receive any `ToLuaMulti` arguments, here '()' and
        // return any `FromLuaMulti` arguments, here a `usize`
        // check the Rlua documentation for more details

        let ctx = ctx.lock().unwrap();

        // equivalent to ctx.globals().set() but for multiple items
        tealr::mlu::set_global_env::<Export>(&ctx).unwrap();

        Ok(())   
    }

    fn get_doc_fragment(&self) -> Option<Self::DocTarget>{
        Some(
            LuaDocFragment::new(
                |tw|
                    // we must select items we want included in the documentation
                    tw.process_type::<APIModule>()
                    .document_global_instance::<Export>().unwrap()
                    
            )
        )
    }

    
}

fn load_our_script(server: Res<AssetServer>, mut commands: Commands) {
    let path = "scripts/teal_file.tl";
    let handle = server.load::<LuaFile, &str>(path);

    commands.spawn().insert(ScriptCollection::<LuaFile> {
        scripts: vec![Script::<LuaFile>::new::<
            RLuaScriptHost<MyLuaArg>,
        >(path.to_string(), handle)],
    });
}

fn fire_our_script(mut w: PriorityEventWriter<LuaEvent<MyLuaArg>>) {
    w.send(LuaEvent::<MyLuaArg>{
        hook_name: "on_update".to_string(),
        args: vec![MyLuaArg],
        recipients: Recipients::All,
    }, 0)
}

fn main() -> std::io::Result<()> {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugin(ScriptingPlugin)
        .add_plugin(ConsolePlugin)
        .add_script_host::<RLuaScriptHost<MyLuaArg>, _>(CoreStage::PostUpdate)
        .add_api_provider::<RLuaScriptHost<MyLuaArg>>(Box::new(LuaAPIProvider))
        // this needs to be placed after any `add_api_provider` and `add_script_host` calls
        // it will generate `doc` and `types` folders under `assets/scripts` containing the documentation and teal declaration files
        // respectively. See example asset folder to see how they look like. The `teal_file.tl` script in example assets shows the usage of one of those 
        // declaration files, use the teal vscode extension to explore the type hints!

        // Note: This is a noop in optimized builds unless the `doc_always` feature is enabled!
        .update_documentation::<RLuaScriptHost<MyLuaArg>>()
        .add_script_handler_stage::<RLuaScriptHost<MyLuaArg>, _, 0, 0>(
            CoreStage::PostUpdate,
        )
        .add_startup_system(load_our_script)
        .add_system(fire_our_script);

    app.run();

    Ok(())
}
