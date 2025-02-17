pub struct CreateUserUseCase {}

pub struct CreateUserInput {}

pub struct CreateUserOutput {}

impl CreateUserUseCase {
    pub async fn invoke(input: CreateUserInput) -> Result<CreateUserOutput, ()> {
        todo!()
    }
}

pub struct GetCurrentUserUseCase {}

pub struct GetCurrentUserInput {}

pub struct GetCurrentUserOutput {}

impl GetCurrentUserUseCase {
    pub async fn invoke(input: GetCurrentUserInput) -> Result<GetCurrentUserOutput, ()> {
        todo!()
    }
}

pub struct LoginUseCase {}

pub struct LoginInput {}

pub struct LoginOutput {}

impl LoginUseCase {
    pub async fn invoke(input: LoginInput) -> Result<LoginOutput, ()> {
        todo!()
    }
}

pub struct UpdateCurrentUserUseCase {}

pub struct UpdateCurrentUserInput {}

pub struct UpdateCurrentUserOutput {}

impl UpdateCurrentUserUseCase {
    pub async fn invoke(input: UpdateCurrentUserInput) -> Result<UpdateCurrentUserOutput, ()> {
        todo!()
    }
}
