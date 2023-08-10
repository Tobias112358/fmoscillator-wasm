use wasm_bindgen::prelude::*;
//use wasm_bindgen::JsCast;
use web_sys;
use web_sys::{AudioContext, OscillatorType};
//use web_sys::{MidiInput, Navigator, MidiAccess, MidiInputMap, Window};
use js_sys::{Promise};
use wasm_bindgen_futures::JsFuture;

//pub use wasm_bindgen_rayon::init_thread_pool;


#[wasm_bindgen(module = "/src/js_modules/sleep.js")]
extern "C" {
    fn sleep(ms: i32) -> Promise;
}

pub async fn timer(ms: i32) -> Result<(), JsValue> {
    let promise = sleep(ms);
    let js_fut = JsFuture::from(promise);
    js_fut.await?;
    Ok(())
}


#[wasm_bindgen]
extern {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hellooooo, {}!", name));
}

/// Converts a midi note to frequency
///
/// A midi note is an integer, generally in the range of 21 to 108
pub fn midi_to_freq(note: u8) -> f32 {
    27.5 * 2f32.powf((note as f32 - 21.0) / 12.0)
}

#[wasm_bindgen]
    pub async fn start_sequence(mut fm: FmOsc){
        while fm.sequencer_mode {

            fm.set_primary_frequency(fm.sequence[fm.step]);
            fm.set_step((fm.step + 1) % 16);

            let _ = timer((30000.0/fm.get_tempo()) as i32).await;
        }

    }

#[wasm_bindgen]
pub struct FmOsc {
    ctx: AudioContext,
    /// The primary oscillator.  This will be the fundamental frequency
    primary: web_sys::OscillatorNode,

    /// Overall gain (volume) control
    gain: web_sys::GainNode,

    /// Amount of frequency modulation
    fm_gain: web_sys::GainNode,

    /// The oscillator that will modulate the primary oscillator's frequency
    fm_osc: web_sys::OscillatorNode,

    /// The ratio between the primary frequency and the fm_osc frequency.
    ///
    /// Generally fractional values like 1/2 or 1/4 sound best
    fm_freq_ratio: f32,

    fm_gain_ratio: f32,

    lfo: web_sys::OscillatorNode,
    lfo_gain: web_sys::GainNode,

    sequencer_mode: bool,
    tempo: f32,
    sequence: [f32; 16],
    step: usize,

    primary_on: bool,
    frequency_on: bool,
    lfo_on: bool,
}

impl Drop for FmOsc {
    fn drop(&mut self) {
        let _ = self.ctx.close();
    }
}

#[wasm_bindgen]
impl FmOsc {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<FmOsc, JsValue> {
        let ctx = web_sys::AudioContext::new()?;

        // Create our web audio objects.
        let primary = ctx.create_oscillator()?;
        let fm_osc = ctx.create_oscillator()?;
        let lfo = ctx.create_oscillator()?;

        let gain = ctx.create_gain()?;
        let fm_gain = ctx.create_gain()?;
        let lfo_gain = ctx.create_gain()?;

        let sequencer_mode = false;
        let tempo = 120.0;
        let sequence = [440.0, 880.0,440.0, 880.0,440.0, 880.0,440.0, 880.0,440.0, 880.0,440.0, 880.0,440.0, 880.0,440.0, 880.0];
        let step = 15;

        // Some initial settings:
        primary.set_type(OscillatorType::Sine);
        primary.frequency().set_value(sequence[step]);
        gain.gain().set_value(0.0); // starts muted

        fm_osc.set_type(OscillatorType::Triangle);
        fm_osc.frequency().set_value(0.0);
        fm_gain.gain().set_value(0.0); // no initial frequency modulation

        lfo.set_type(OscillatorType::Sine);
        lfo.frequency().set_value(2.0);
        lfo_gain.gain().set_value(0.0);

        // Connect the nodes up!

        //lfo connections
        lfo.connect_with_audio_param(&gain.gain())?;
        lfo.connect_with_audio_node(&lfo_gain)?;

        // The primary oscillator is routed through the gain node, so that
        // it can control the overall output volume.
        primary.connect_with_audio_node(&gain)?;

        // Then connect the gain node to the AudioContext destination (aka
        // your speakers).
        gain.connect_with_audio_node(&ctx.destination())?;

        // The FM oscillator is connected to its own gain node, so it can
        // control the amount of modulation.
        fm_osc.connect_with_audio_node(&fm_gain)?;

        // Connect the FM oscillator to the frequency parameter of the main
        // oscillator, so that the FM node can modulate its frequency.
        fm_gain.connect_with_audio_param(&primary.frequency())?;

        // Start the oscillators!
        //primary.start()?;
        //fm_osc.start()?;
        //lfo.start()?;  
        let primary_on = false;
        let frequency_on = false;
        let lfo_on = false;

        Ok(FmOsc {
            ctx,
            primary,
            gain,
            fm_gain,
            fm_osc,
            fm_freq_ratio: 0.0,
            fm_gain_ratio: 0.0,
            lfo,
            lfo_gain,
            sequencer_mode,
            tempo,
            sequence,
            step,
            primary_on,
            frequency_on,
            lfo_on
        })
    }

    #[wasm_bindgen]
    pub async fn start_sequence(&mut self){
        while self.sequencer_mode {

            self.set_primary_frequency(self.sequence[self.step]);
            self.set_step((self.step + 1) % 16);

            let _ = timer((30000.0/self.get_tempo()) as i32).await;            
        }

    }

    #[wasm_bindgen]
    pub fn stop_sequence(&self) -> usize{
        self.step
    }

    #[wasm_bindgen]
    pub fn start_primary_oscillator(&mut self) {
        let _ = self.primary.start();
        self.primary_on = true;
    }

    #[wasm_bindgen]
    pub fn stop_primary_oscillator(&mut self) {
        let _ = self.primary.stop();
        self.primary_on = false;
    }

    #[wasm_bindgen]
    pub fn toggle_primary_oscillator(&mut self) -> Result<(), JsValue> {
        let is_on = self.get_primary_is_on();
        if is_on {
            let _ = self.primary.stop();
        } else {
            self.primary = self.ctx.create_oscillator()?;
            self.primary.set_type(OscillatorType::Sine);
            self.primary.frequency().set_value(self.sequence[self.step]);
            self.primary.connect_with_audio_node(&self.gain)?;
            self.fm_gain.connect_with_audio_param(&self.primary.frequency())?;
            let _ = self.primary.start();
        }
        self.primary_on = !is_on;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn start_frequency_oscillator(&mut self) {
        let _ = self.fm_osc.start();
        self.frequency_on = true;
    }

    #[wasm_bindgen]
    pub fn stop_frequency_oscillator(&mut self) {
        let _ = self.fm_osc.stop();
        self.frequency_on = false;
    }

    #[wasm_bindgen]
    pub fn toggle_frequency_oscillator(&mut self) -> Result<(), JsValue> {
        let is_on = self.get_frequency_is_on();
        if is_on {
            let _ = self.fm_osc.stop();
        } else {
            self.fm_osc = self.ctx.create_oscillator()?;
            self.fm_osc.set_type(OscillatorType::Triangle);
            self.fm_osc.frequency().set_value(1.0);
            self.fm_osc.connect_with_audio_node(&self.fm_gain)?;
            let _ = self.fm_osc.start();
        }
        self.frequency_on = !is_on;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn start_lfo(&mut self) {
        let _ = self.lfo.start();
        self.lfo_on = true;
    }

    #[wasm_bindgen]
    pub fn stop_lfo(&mut self) {
        let _ = self.lfo.stop();
        self.lfo_on = false;
    }

    #[wasm_bindgen]
    pub fn toggle_lfo(&mut self) -> Result<(), JsValue> {
        let is_on = self.get_lfo_is_on();
        if is_on {
            let _ = self.lfo.stop();
        } else {
            self.lfo = self.ctx.create_oscillator()?;
            self.lfo.set_type(OscillatorType::Sine);
            self.lfo.frequency().set_value(2.0);
            self.lfo.connect_with_audio_param(&self.gain.gain())?;
            self.lfo.connect_with_audio_node(&self.lfo_gain)?;
            let _ = self.lfo.start();
        }
        self.lfo_on = !is_on;
        Ok(())
    }

    #[wasm_bindgen]
    pub fn toggle_sequencer_mode(&mut self) -> bool{
        self.sequencer_mode = !self.sequencer_mode;
        self.sequencer_mode
    }

    /// Getters
    #[wasm_bindgen]
    pub fn get_step(&self) -> usize{
        self.step
    }

    #[wasm_bindgen]
    pub fn get_tempo(&self) -> f32{
        self.tempo
    }

    #[wasm_bindgen]
    pub fn get_primary_is_on(&self) -> bool{
        self.primary_on
    }

    #[wasm_bindgen]
    pub fn get_frequency_is_on(&self) -> bool{
        self.frequency_on
    }

    #[wasm_bindgen]
    pub fn get_lfo_is_on(&self) -> bool{
        self.lfo_on
    }

    /*#[wasm_bindgen]
    pub fn get_midi_access(&self) -> web_sys::MidiInputMap{
        self.midi_access.inputs()
    }*/

    /// Sets the gain for this oscillator, between 0.0 and 1.0.
    #[wasm_bindgen]
    pub fn set_gain(&self, mut gain: f32) {
        if gain > 1.0 {
            gain = 1.0;
        }
        if gain < 0.0 {
            gain = 0.0;
        }
        self.gain.gain().set_value(gain);
    }

    #[wasm_bindgen]
    pub fn set_tempo(&mut self, tempo: f32) {
        self.tempo = (tempo*10.0).round() / 10.0;
    }

    #[wasm_bindgen]
    pub fn set_step(&mut self, step: usize) {
        self.step = step;
    }

    #[wasm_bindgen]
    pub fn set_primary_frequency(&self, freq: f32) {
        self.primary.frequency().set_value(freq);

        // The frequency of the FM oscillator depends on the frequency of the
        // primary oscillator, so we update the frequency of both in this method.   
        self.fm_osc.frequency().set_value(self.fm_freq_ratio * freq);
        self.fm_gain.gain().set_value(self.fm_gain_ratio * freq);
    }

    #[wasm_bindgen]
    pub fn set_note(&self, note: u8) {
        let freq = midi_to_freq(note);
        self.set_primary_frequency(freq);
    }

    /// This should be between 0 and 1, though higher values are accepted.
    #[wasm_bindgen]
    pub fn set_fm_amount(&mut self, amt: f32) {
        self.fm_gain_ratio = amt;

        self.fm_gain
            .gain()
            .set_value(self.fm_gain_ratio * self.primary.frequency().value());
    }

    /// This should be between 0 and 1, though higher values are accepted.
    #[wasm_bindgen]
    pub fn set_fm_frequency(&mut self, amt: f32) {
        self.fm_freq_ratio = amt;
        self.fm_osc
            .frequency()
            .set_value(self.fm_freq_ratio * self.primary.frequency().value());
    }

    /// This should be between 0 and 1, though higher values are accepted.
    #[wasm_bindgen]
    pub fn set_primary_oscillator_type(&mut self, wave: &str) {
        match wave{
            "sine"=>self.primary.set_type(OscillatorType::Sine),
            "triangle"=>self.primary.set_type(OscillatorType::Triangle),
            "sawtooth"=>self.primary.set_type(OscillatorType::Sawtooth),
            "square"=>self.primary.set_type(OscillatorType::Square),
            &_=>self.primary.set_type(OscillatorType::Sine),
        }
    }

    #[wasm_bindgen]
    pub fn set_fm_oscillator_type(&mut self, wave: &str) {
        match wave{
            "sine"=>self.fm_osc.set_type(OscillatorType::Sine),
            "triangle"=>self.fm_osc.set_type(OscillatorType::Triangle),
            "sawtooth"=>self.fm_osc.set_type(OscillatorType::Sawtooth),
            "square"=>self.fm_osc.set_type(OscillatorType::Square),
            &_=>self.fm_osc.set_type(OscillatorType::Sine),
        }
    }

    #[wasm_bindgen]
    pub fn set_lfo_oscillator_type(&mut self, wave: &str) {
        match wave{
            "sine"=>self.lfo.set_type(OscillatorType::Sine),
            "triangle"=>self.lfo.set_type(OscillatorType::Triangle),
            "sawtooth"=>self.lfo.set_type(OscillatorType::Sawtooth),
            "square"=>self.lfo.set_type(OscillatorType::Square),
            &_=>self.lfo.set_type(OscillatorType::Sine),
        }
    }

    #[wasm_bindgen]
    pub fn set_lfo_frequency(&mut self, freq: f32) {
        self.lfo.frequency().set_value(freq);
    }

    #[wasm_bindgen]
    pub fn set_lfo_amplitude(&mut self, mut gain: f32) {
        if gain > 1.0 {
            gain = 1.0;
        }
        if gain < 0.0 {
            gain = 0.0;
        }
        self.lfo_gain.gain().set_value(gain);
    }

    #[wasm_bindgen]
    pub fn sync_lfo_with_tempo(&mut self) {
        
        self.lfo.frequency().set_value(self.tempo/60.0);
    }

    #[wasm_bindgen]
    pub fn set_sequence(&mut self, frequencies: &[f32]) {
        let mut count = 0;
        for frequency in frequencies {
            self.sequence[count] = *frequency;
            count = count + 1;
        }
    }

    #[wasm_bindgen]
    pub fn set_sequence_step(&mut self, frequency: f32, step: usize) {
        
        self.sequence[step] = frequency;
    }

    #[wasm_bindgen]
    pub fn next_step(&mut self) {
            self.step = (self.step + 1) % 16;
        self.primary.frequency().set_value(self.sequence[usize::from(self.step)]);
    }
}
