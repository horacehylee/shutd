package shutd

import (
	"context"
	"errors"
	"fmt"
	"time"
)

func newNotificationSnoozeTask() SchedulerTask {
	return func(s *Scheduler) error {
		shutdownTime, err := s.ShutdownTime()
		if err != nil {
			return err
		}
		title := fmt.Sprintf("Shutd - Scheduled Shutdown at %v", shutdownTime.Format("15:04"))
		text := fmt.Sprintf("About to shutdown in %.0f minutes, wanted to snooze for %v minutes?", time.Until(shutdownTime).Minutes(), s.Config().SnoozeInterval)

		ctx, cancel := context.WithTimeout(context.Background(), time.Duration(s.Config().Notification.Duration)*time.Minute)
		defer cancel()
		yes, err := question(ctx, title, text)
		if err != nil && !errors.Is(err, context.Canceled) && !errors.Is(err, context.DeadlineExceeded) {
			return fmt.Errorf("failed to display snooze notification: %v", err)
		}
		s.Logger().Infof("snooze is required: %v", yes)
		if yes {
			err := s.Snooze()
			if err != nil {
				return err
			}
		}
		return nil
	}
}
