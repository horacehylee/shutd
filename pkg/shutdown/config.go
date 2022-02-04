package shutdown

type Config struct {
	SnoozeInterval int
	StartTime      string
	Notification   struct {
		Before   int
		Duration int
	}
}
