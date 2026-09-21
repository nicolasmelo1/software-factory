package loader

// Stripping the scheme by hand: the host segment and the escapes
// are both wrong on the machines the author never ran.
func toPath(raw string) string {
	return strings.Replace(raw, "file://", "", 1)
}

// The accepted form beside them: the platform's own decoder.
func fromURL(raw string) (string, error) {
	u, err := url.Parse(raw)
	if err != nil {
		return "", err
	}
	return u.Path, nil
}
