package s3

import (
	"context"
	"os"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
)

type FilePresigner struct {
	S3PresignClient *s3.PresignClient
	BucketName      string
}

func NewFilePresigner() (*FilePresigner, error) {
	endpoint := os.Getenv("S3_ENDPOINT")
	region := os.Getenv("AWS_REGION")
	bucketName := os.Getenv("S3_BUCKET_NAME")
	accessKey := os.Getenv("AWS_ACCESS_KEY_ID")
	secretKey := os.Getenv("AWS_SECRET_ACCESS_KEY")
	usePathStyle := os.Getenv("S3_USE_PATH_STYLE") == "true"

	resolver := aws.EndpointResolverWithOptionsFunc(func(service, region string, options ...interface{}) (aws.Endpoint, error) {
		return aws.Endpoint{
			URL:               endpoint,
			SigningRegion:     region,
			HostnameImmutable: true,
		}, nil
	})

	cfg, err := config.LoadDefaultConfig(
		context.TODO(),
		config.WithRegion(region),
		config.WithEndpointResolverWithOptions(resolver),
		config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider(accessKey, secretKey, "")),
	)

	if err != nil {
		return nil, err
	}

	s3Client := s3.NewFromConfig(cfg, func(o *s3.Options) {
		o.UsePathStyle = usePathStyle
	})

	presignClient := s3.NewPresignClient(s3Client)

	return &FilePresigner{
		S3PresignClient: presignClient,
		BucketName:      bucketName,
	}, nil
}

func (p *FilePresigner) GeneratePresignedUploadURL(objectKey string) (string, error) {
	request, err := p.S3PresignClient.PresignPutObject(
		context.TODO(),
		&s3.PutObjectInput{
			Bucket: aws.String(p.BucketName),
			Key:    aws.String(objectKey),
		},
		func(opts *s3.PresignOptions) {
			opts.Expires = time.Duration(15 * time.Minute)
		},
	)

	if err != nil {
		return "", err
	}

	return request.URL, nil
}
