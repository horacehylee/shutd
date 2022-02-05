package shutdown

// Config for shutdown scheduler
type Config struct {
	SnoozeInterval int
	StartTime      string
	Notification   struct {
		Before   int
		Duration int
	}
}
