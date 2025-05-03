use bin_packets::IcarusPhase;
use heapless::Vec;
use rtic_sync::signal::{SignalReader, SignalWriter};

pub struct StateMachine<const N: usize> {
    state: IcarusPhase,
    channels: Vec<SignalWriter<'static, IcarusPhase>, N>,
}

impl<const N: usize> Default for StateMachine<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> StateMachine<N> {
    pub fn new() -> Self {
        Self {
            state: IcarusPhase::Ejection,
            channels: Vec::new(),
        }
    }

    pub fn get_phase(&self) -> IcarusPhase {
        self.state
    }

    fn set_state(&mut self, state: IcarusPhase) {
        self.state = state;

        // Notify
        self.channels.iter_mut().for_each(|writer| {
            writer.write(state);
        });
    }

    pub fn add_channel(&mut self, channel: SignalWriter<'static, IcarusPhase>) -> Result<(), ()> {
        match self.channels.push(channel) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }
}

pub struct StateMachineListener {
    reader: SignalReader<'static, IcarusPhase>,
}

impl StateMachineListener {
    pub fn new(reader: SignalReader<'static, IcarusPhase>) -> Self {
        Self { reader }
    }

    async fn wait_for_state(&mut self) -> IcarusPhase {
        self.reader.wait().await
    }

    /// Waits for a specific state to be entered
    pub async fn wait_for_state_specific(&mut self, state: IcarusPhase) {
        loop {
            let current_state = self.wait_for_state().await;
            if current_state == state {
                return;
            }
        }
    }
}
