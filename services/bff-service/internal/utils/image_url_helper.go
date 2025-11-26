package utils

import (
	"fmt"
	"strings"
)

func RewriteAvatarURL(internalURL string, baseURL string) string {
	internalHost := "http://minio:9000/"

	if !strings.HasPrefix(internalURL, internalHost) {
		return internalURL
	}

	objectKey := strings.TrimPrefix(internalURL, internalHost)

	return fmt.Sprintf("%s/v1/proxy/image?key=%s", baseURL, objectKey)
}
