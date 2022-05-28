package shutd

import (
	"fmt"
	"os/exec"
)

func newShutdownTask() SchedulerTask {
	return func(s *Scheduler) error {
		return execShutdown()
	}
}

func execShutdown() (err error) {
	if err := exec.Command("cmd", "/C", "shutdown", "/sg", "/t", "0").Run(); err != nil {
		return fmt.Errorf("failed to initiate shutdown: %w", err)
	}
	return nil
}
