pub struct CreateArticleCommentUseCase {}

pub struct CreateArticleCommentInput {}

pub struct CreateArticleCommentOutput {}

impl CreateArticleCommentUseCase {
    pub async fn invoke(
        input: CreateArticleCommentInput,
    ) -> Result<CreateArticleCommentOutput, ()> {
        todo!()
    }
}

pub struct DeleteArticleCommentUseCase {}

pub struct DeleteArticleCommentInput {}

pub struct DeleteArticleCommentOutput {}

impl DeleteArticleCommentUseCase {
    pub async fn invoke(
        input: DeleteArticleCommentInput,
    ) -> Result<DeleteArticleCommentOutput, ()> {
        todo!()
    }
}

pub struct GetArticleCommentsUseCase {}

pub struct GetArticleCommentsInput {}

pub struct GetArticleCommentsOutput {}

impl GetArticleCommentsUseCase {
    pub async fn invoke(input: GetArticleCommentsInput) -> Result<GetArticleCommentsOutput, ()> {
        todo!()
    }
}
