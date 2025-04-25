use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct EventPayload {
    event_name: String,
    data: serde_json::Value
}

pub struct EventEmitter {
    pub sender: broadcast::Sender<EventPayload>
}

impl EventEmitter {

    pub fn new(size: usize) -> Self {
        let (sender, _rv) = broadcast::channel::<EventPayload>(size);

        Self {
            sender
        }
    }

    pub fn on<T, P>(&self, event_name: &str, callback: fn(T) -> Result<P, Box<dyn std::error::Error>>)
    where
        T: serde::de::DeserializeOwned + Send + 'static,
        P: Send + 'static
    {
        let mut subscriber = self.sender.subscribe();
        let event_name = event_name.to_string();

        tokio::spawn(async move {
            while let Ok(payload) = subscriber.recv().await {
                if payload.event_name == event_name {
                    match serde_json::from_value::<T>(payload.data.clone()) {
                        Ok(parsed) => {
                            if let Err(e) = callback(parsed){
                                eprintln!("Error: {:?}", e);
                            }
                        },
                        Err(e) => eprintln!("Error: {:?}", e)
                    }
                }

            }
        });
    }

    pub fn emit<T>(&self, event_name: &str, data: T)
    where
        T: serde::Serialize,
    {
        let payload = EventPayload {
            event_name: event_name.to_string(),
            data: serde_json::to_value(data).unwrap()
        };
        self.sender.send(payload).unwrap();
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_event_emitter() {
        let chat = crate::utils::EventEmitter::new(16);

        chat.on("message", |payload: String| {
            println!("{:?}", payload);
            Ok(())
        });

        chat.emit("message", "Hello World");

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
}