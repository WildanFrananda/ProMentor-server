package jwt

import (
	"auth-service/internal/model"
	"os"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

func GenerateTokens(user *model.User) (accessToken string, refreshToken string, err error) {
	jwtSecret := []byte(os.Getenv("JWT_SECRET"))

	accessClaims := jwt.MapClaims{
		"sub":   user.ID,
		"name":  user.Name,
		"email": user.Email,
		"role":  user.Role,
		"exp":   time.Now().Add(time.Minute * 15).Unix(),
	}
	accessToken, err = jwt.NewWithClaims(jwt.SigningMethodHS256, accessClaims).SignedString(jwtSecret)
	if err != nil {
		return "", "", err
	}

	refreshClaims := jwt.MapClaims{
		"sub": user.ID,
		"exp": time.Now().Add(time.Hour * 24 * 30).Unix(),
	}
	refreshToken, err = jwt.NewWithClaims(jwt.SigningMethodHS256, refreshClaims).SignedString(jwtSecret)
	if err != nil {
		return "", "", err
	}

	return accessToken, refreshToken, nil
}

func ValidateToken(tokenString string) (jwt.MapClaims, error) {
	jwtSecret := []byte(os.Getenv("JWT_SECRET"))

	token, err := jwt.Parse(tokenString, func(token *jwt.Token) (interface{}, error) {
		if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, jwt.ErrSignatureInvalid
		}
		return jwtSecret, nil
	})

	if err != nil {
		return nil, err
	}

	if claims, ok := token.Claims.(jwt.MapClaims); ok && token.Valid {
		return claims, nil
	}

	return nil, jwt.ErrInvalidKey
}
