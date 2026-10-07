use crate::commands::Command;

/// The lilac post-it: "whenever `when` holds, run the command `then` returns".
///
/// The policy never runs the command itself: it only says which one should run, so the domain stays free of IO.
/// The command has `Output = ()`: nobody is there to receive what a command run by a policy gives back.
pub struct Policy<Ports> {
    name: &'static str,
    when: Box<dyn Fn() -> bool + Send + Sync>,
    then: Box<dyn Fn() -> Box<dyn Command<Ports, Output = ()>> + Send + Sync>,
}

impl<Ports> Policy<Ports> {
    pub fn new(
        name: &'static str,
        when: impl Fn() -> bool + Send + Sync + 'static,
        then: impl Fn() -> Box<dyn Command<Ports, Output = ()>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name,
            when: Box::new(when),
            then: Box::new(then),
        }
    }
}

/// A policy that fired, with the command it wants to run.
pub struct FiredPolicy<Ports> {
    pub name: &'static str,
    pub command: Box<dyn Command<Ports, Output = ()>>,
}

pub struct Policies<Ports>(Vec<Policy<Ports>>);

impl<Ports> Policies<Ports> {
    pub fn new(policies: Vec<Policy<Ports>>) -> Self {
        Self(policies)
    }

    /// Fires every policy whose `when` holds; returns each one with the command its `then` built.
    pub fn trigger(&self) -> Vec<FiredPolicy<Ports>> {
        self.0
            .iter()
            .filter(|p| (p.when)())
            .map(|p| FiredPolicy {
                name: p.name,
                command: (p.then)(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::Executed;
    use crate::errors::Error;
    use async_trait::async_trait;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Ports {
        shipped: Mutex<Vec<u64>>,
    }

    struct ShipOrderCommand {
        order_id: u64,
    }

    #[async_trait]
    impl Command<Ports> for ShipOrderCommand {
        type Output = ();

        async fn execute(&self, ports: &Ports) -> Result<Executed<(), Ports>, Error> {
            ports.shipped.lock().unwrap().push(self.order_id);

            Ok(Executed {
                output: (),
                events: vec![],
            })
        }
    }

    fn ship(order_id: u64) -> Box<dyn Command<Ports, Output = ()>> {
        Box::new(ShipOrderCommand { order_id })
    }

    #[test]
    fn trigger_fires_only_policies_whose_when_holds() {
        let is_admin = true;

        let policies = Policies::new(vec![
            Policy::new("send welcome email", || true, || ship(1)),
            Policy::new("request admin approval", move || is_admin, || ship(2)),
            Policy::new("never", || false, || ship(3)),
        ]);

        let fired: Vec<_> = policies.trigger().into_iter().map(|f| f.name).collect();

        assert_eq!(fired, vec!["send welcome email", "request admin approval"]);
    }

    #[tokio::test]
    async fn trigger_returns_the_command_built_by_then() {
        let ports = Ports::default();
        let order_id = 42;

        let policies = Policies::new(vec![Policy::new(
            "ship order",
            || true,
            move || ship(order_id),
        )]);

        for fired in policies.trigger() {
            fired.command.execute(&ports).await.unwrap();
        }

        assert_eq!(*ports.shipped.lock().unwrap(), vec![42]);
    }
}
