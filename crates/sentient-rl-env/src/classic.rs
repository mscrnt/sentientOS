//! Classic control environments

use async_trait::async_trait;
use rand::Rng;

use sentient_rl_core::{
    Environment, EnvironmentConfig, Step, StepInfo,
    BoxSpace, DiscreteSpace, VectorState, VectorObservation,
    DiscreteAction, ContinuousAction, Reward, Terminal,
    BoxObservationSpace, RLError, Result,
};

/// CartPole environment
pub struct CartPoleEnv {
    /// Current state
    state: CartPoleState,
    /// Configuration
    config: CartPoleConfig,
    /// Step count
    steps: usize,
}

#[derive(Debug, Clone)]
struct CartPoleState {
    x: f64,          // Cart position
    x_dot: f64,      // Cart velocity
    theta: f64,      // Pole angle
    theta_dot: f64,  // Pole angular velocity
}

#[derive(Debug, Clone)]
struct CartPoleConfig {
    gravity: f64,
    mass_cart: f64,
    mass_pole: f64,
    length: f64,
    force_mag: f64,
    max_steps: usize,
    x_threshold: f64,
    theta_threshold: f64,
}

impl Default for CartPoleConfig {
    fn default() -> Self {
        Self {
            gravity: 9.8,
            mass_cart: 1.0,
            mass_pole: 0.1,
            length: 0.5,
            force_mag: 10.0,
            max_steps: 500,
            x_threshold: 2.4,
            theta_threshold: 0.209, // ~12 degrees
        }
    }
}

impl CartPoleEnv {
    /// Create a new CartPole environment
    pub fn new(config: EnvironmentConfig) -> Result<Self> {
        Ok(Self {
            state: CartPoleState {
                x: 0.0,
                x_dot: 0.0,
                theta: 0.0,
                theta_dot: 0.0,
            },
            config: CartPoleConfig::default(),
            steps: 0,
        })
    }
    
    fn get_observation(&self) -> VectorObservation {
        VectorObservation {
            data: vec![
                self.state.x,
                self.state.x_dot,
                self.state.theta,
                self.state.theta_dot,
            ],
        }
    }
    
    fn is_done(&self) -> bool {
        self.state.x.abs() > self.config.x_threshold ||
        self.state.theta.abs() > self.config.theta_threshold ||
        self.steps >= self.config.max_steps
    }
}

#[async_trait]
impl Environment for CartPoleEnv {
    type Observation = VectorObservation;
    type Action = DiscreteAction;
    type State = VectorState;
    
    fn observation_space(&self) -> Box<dyn sentient_rl_core::ObservationSpace<Observation = Self::Observation>> {
        let high = vec![
            self.config.x_threshold * 2.0,
            f64::INFINITY,
            self.config.theta_threshold * 2.0,
            f64::INFINITY,
        ];
        let low = high.iter().map(|&x| -x).collect();
        
        Box::new(BoxObservationSpace::new(low, high, vec![4]).unwrap())
    }
    
    fn action_space(&self) -> Box<dyn sentient_rl_core::ActionSpace<Action = Self::Action>> {
        Box::new(DiscreteSpace::new(2)) // 0: push left, 1: push right
    }
    
    async fn reset(&mut self) -> Result<(Self::Observation, StepInfo)> {
        let mut rng = rand::thread_rng();
        self.state = CartPoleState {
            x: rng.gen_range(-0.05..0.05),
            x_dot: rng.gen_range(-0.05..0.05),
            theta: rng.gen_range(-0.05..0.05),
            theta_dot: rng.gen_range(-0.05..0.05),
        };
        self.steps = 0;
        
        Ok((self.get_observation(), StepInfo::default()))
    }
    
    async fn step(&mut self, action: Self::Action) -> Result<Step<Self::Observation, Self::State>> {
        // Physics simulation
        let force = if action.0 == 1 {
            self.config.force_mag
        } else {
            -self.config.force_mag
        };
        
        let cos_theta = self.state.theta.cos();
        let sin_theta = self.state.theta.sin();
        
        let total_mass = self.config.mass_cart + self.config.mass_pole;
        let pole_mass_length = self.config.mass_pole * self.config.length;
        
        let temp = (force + pole_mass_length * self.state.theta_dot.powi(2) * sin_theta) / total_mass;
        let theta_acc = (self.config.gravity * sin_theta - cos_theta * temp) /
            (self.config.length * (4.0 / 3.0 - self.config.mass_pole * cos_theta.powi(2) / total_mass));
        let x_acc = temp - pole_mass_length * theta_acc * cos_theta / total_mass;
        
        // Update state
        let dt = 0.02; // Time step
        self.state.x += dt * self.state.x_dot;
        self.state.x_dot += dt * x_acc;
        self.state.theta += dt * self.state.theta_dot;
        self.state.theta_dot += dt * theta_acc;
        
        self.steps += 1;
        
        let done = self.is_done();
        let truncated = self.steps >= self.config.max_steps && !done;
        
        Ok(Step {
            observation: self.get_observation(),
            reward: Reward(1.0), // Reward of 1 for each step survived
            done,
            truncated,
            info: StepInfo::default(),
            state: Some(VectorState {
                data: vec![self.state.x, self.state.x_dot, self.state.theta, self.state.theta_dot],
                terminal: if done { Terminal::Yes } else { Terminal::No },
            }),
        })
    }
}

/// Mountain Car environment
pub struct MountainCarEnv {
    /// Current state
    state: MountainCarState,
    /// Configuration
    config: MountainCarConfig,
    /// Step count
    steps: usize,
}

#[derive(Debug, Clone)]
struct MountainCarState {
    position: f64,
    velocity: f64,
}

#[derive(Debug, Clone)]
struct MountainCarConfig {
    min_position: f64,
    max_position: f64,
    max_speed: f64,
    goal_position: f64,
    goal_velocity: f64,
    force: f64,
    gravity: f64,
    max_steps: usize,
}

impl Default for MountainCarConfig {
    fn default() -> Self {
        Self {
            min_position: -1.2,
            max_position: 0.6,
            max_speed: 0.07,
            goal_position: 0.5,
            goal_velocity: 0.0,
            force: 0.001,
            gravity: 0.0025,
            max_steps: 200,
        }
    }
}

impl MountainCarEnv {
    /// Create a new Mountain Car environment
    pub fn new(config: EnvironmentConfig) -> Result<Self> {
        Ok(Self {
            state: MountainCarState {
                position: -0.5,
                velocity: 0.0,
            },
            config: MountainCarConfig::default(),
            steps: 0,
        })
    }
}

#[async_trait]
impl Environment for MountainCarEnv {
    type Observation = VectorObservation;
    type Action = DiscreteAction;
    type State = VectorState;
    
    fn observation_space(&self) -> Box<dyn sentient_rl_core::ObservationSpace<Observation = Self::Observation>> {
        Box::new(BoxObservationSpace::new(
            vec![self.config.min_position, -self.config.max_speed],
            vec![self.config.max_position, self.config.max_speed],
            vec![2],
        ).unwrap())
    }
    
    fn action_space(&self) -> Box<dyn sentient_rl_core::ActionSpace<Action = Self::Action>> {
        Box::new(DiscreteSpace::new(3)) // 0: push left, 1: no push, 2: push right
    }
    
    async fn reset(&mut self) -> Result<(Self::Observation, StepInfo)> {
        let mut rng = rand::thread_rng();
        self.state = MountainCarState {
            position: rng.gen_range(-0.6..-0.4),
            velocity: 0.0,
        };
        self.steps = 0;
        
        Ok((
            VectorObservation {
                data: vec![self.state.position, self.state.velocity],
            },
            StepInfo::default(),
        ))
    }
    
    async fn step(&mut self, action: Self::Action) -> Result<Step<Self::Observation, Self::State>> {
        // Convert action to force
        let force = match action.0 {
            0 => -1.0,
            1 => 0.0,
            2 => 1.0,
            _ => return Err(RLError::InvalidAction(format!("Invalid action: {}", action.0))),
        };
        
        // Update velocity
        self.state.velocity += force * self.config.force + 
            self.state.position.cos() * (-self.config.gravity);
        self.state.velocity = self.state.velocity.clamp(-self.config.max_speed, self.config.max_speed);
        
        // Update position
        self.state.position += self.state.velocity;
        self.state.position = self.state.position.clamp(
            self.config.min_position,
            self.config.max_position,
        );
        
        // Stop at boundaries
        if self.state.position <= self.config.min_position {
            self.state.velocity = 0.0;
        }
        
        self.steps += 1;
        
        // Check if goal reached
        let done = self.state.position >= self.config.goal_position &&
                  self.state.velocity >= self.config.goal_velocity;
        let truncated = self.steps >= self.config.max_steps && !done;
        
        let reward = if done { 0.0 } else { -1.0 };
        
        Ok(Step {
            observation: VectorObservation {
                data: vec![self.state.position, self.state.velocity],
            },
            reward: Reward(reward),
            done,
            truncated,
            info: StepInfo::default(),
            state: Some(VectorState {
                data: vec![self.state.position, self.state.velocity],
                terminal: if done { Terminal::Yes } else { Terminal::No },
            }),
        })
    }
}