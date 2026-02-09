use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Default, Clone, Copy)]
    pub struct KeyState: u16 {
        const MoveForward = 1 << 0;
        const MoveBackward = 1 << 1;
        const MoveRight = 1 << 2;
        const MoveLeft = 1 << 3;
        const MoveUp = 1 << 4;
        const MoveDown = 1 << 5;
        const PitchUp = 1 << 6;
        const PitchDown = 1 << 7;
        const YawRight = 1 << 8;
        const YawLeft = 1 << 9;
        const Shift = 1 << 10;
        const Control = 1 << 11;
    }
}

type Magnitude = i8;

impl KeyState {
    fn magnitude(&self, positive: Self, negative: Self) -> Magnitude {
        Magnitude::from(self.contains(positive)) - Magnitude::from(self.contains(negative))
    }

    pub fn forward_magnitude(&self) -> Magnitude {
        self.magnitude(Self::MoveForward, Self::MoveBackward)
    }

    pub fn right_magnitude(&self) -> Magnitude {
        self.magnitude(Self::MoveRight, Self::MoveLeft)
    }

    pub fn up_magnitude(&self) -> Magnitude {
        self.magnitude(Self::MoveUp, Self::MoveDown)
    }

    pub fn pitch_magnitude(&self) -> Magnitude {
        self.magnitude(Self::PitchDown, Self::PitchUp)
    }

    pub fn yaw_magnitude(&self) -> Magnitude {
        self.magnitude(Self::YawRight, Self::YawLeft)
    }
}
